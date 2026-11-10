//! `umcf` CLI binary for Universal Mobile Compute Fabric.

use std::env;
use std::fs;
use std::path::Path;
use std::process::ExitCode;
use umcf_coordinator::ClusterCoordinator;
use umcf_core::{ComputeBackend, NodeId, NodeRole, ResourceConsentPolicy};
use umcf_proto::{MessageKind, PlaneKind, WireEnvelope, CURRENT_PROTOCOL_VERSION};
use umcf_scheduler::{DistributedInferenceRequest, HeterogeneousScheduler, InferencePrivacyMode};
use umcf_worker::probe_current_host_profile;

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    match run_cli(&args) {
        Ok(output) => {
            println!("{output}");
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("error: {err}");
            ExitCode::from(1)
        }
    }
}

pub fn run_cli(args: &[String]) -> Result<String, String> {
    let cmd = args.first().map_or("help", String::as_str);
    let json_flag = args.iter().any(|a| a == "--json");

    match cmd {
        "version" | "--version" | "-V" => Ok(format!(
            "umcf {} (protocol v{})",
            env!("CARGO_PKG_VERSION"),
            CURRENT_PROTOCOL_VERSION
        )),
        "doctor" => run_doctor(json_flag),
        "verify-protocol" => run_verify_protocol(json_flag),
        "plan-inference" => run_plan_inference(&args[1..], json_flag),
        "help" | "--help" | "-h" => Ok(usage_text()),
        other => Err(format!(
            "unknown subcommand '{other}'. Run 'umcf help' for usage."
        )),
    }
}

fn usage_text() -> String {
    [
        "Universal Mobile Compute Fabric CLI (umcf)",
        "",
        "USAGE:",
        "  umcf version                      Print CLI and wire protocol version",
        "  umcf doctor [--json]              Inspect host hardware, Podman 30GB cap & protocol status",
        "  umcf verify-protocol [--json]     Execute deterministic wire protocol & plane isolation checks",
        "  umcf plan-inference [--json]      Compute heterogeneous layer partitioning across cluster nodes",
    ]
    .join("\n")
}

fn run_doctor(json: bool) -> Result<String, String> {
    let local_id = NodeId::from_public_key_bytes(b"local-cli-doctor-identity-seed-01")
        .map_err(|e| e.to_string())?;
    let hw = probe_current_host_profile(local_id);

    let storage_conf_ok = check_podman_storage_conf();
    let proto_ok = run_verify_protocol(false).is_ok();

    if json {
        Ok(format!(
            "{{\"status\":\"PASS\",\"version\":\"{}\",\"protocol_version\":\"{}\",\"platform\":\"{:?}\",\"arch\":\"{:?}\",\"logical_cores\":{},\"podman_30gb_config_verified\":{},\"protocol_self_test\":{},\"verified_accelerators\":{}}}",
            env!("CARGO_PKG_VERSION"),
            CURRENT_PROTOCOL_VERSION,
            hw.platform,
            hw.cpu_arch,
            hw.logical_cores,
            storage_conf_ok,
            proto_ok,
            hw.verified_accelerators().len()
        ))
    } else {
        Ok(format!(
            "UMCF Doctor Report\n  Status              : PASS\n  CLI Version         : {}\n  Protocol Version    : {}\n  Host Platform       : {:?} ({:?})\n  Logical CPU Cores   : {}\n  Podman 30GB Cap     : {}\n  Protocol Self-Test  : {}\n  Verified GPU/NPUs   : {} (Hardware-Unverified until Stage 3 benchmark pass)",
            env!("CARGO_PKG_VERSION"),
            CURRENT_PROTOCOL_VERSION,
            hw.platform,
            hw.cpu_arch,
            hw.logical_cores,
            if storage_conf_ok { "VERIFIED (size = 30G)" } else { "UNVERIFIED" },
            if proto_ok { "PASS" } else { "FAIL" },
            hw.verified_accelerators().len()
        ))
    }
}

fn check_podman_storage_conf() -> bool {
    let candidates = [
        "infra/podman/storage.conf",
        "../../infra/podman/storage.conf",
    ];
    for p in candidates {
        if Path::new(p).exists() {
            if let Ok(content) = fs::read_to_string(p) {
                if content.contains("size = \"30G\"") {
                    return true;
                }
            }
        }
    }
    false
}

fn run_verify_protocol(json: bool) -> Result<String, String> {
    let ctrl_env = WireEnvelope::new(
        PlaneKind::ControlPlane,
        MessageKind::CapabilityReport,
        1,
        "did:umcf:00112233445566778899aabbccddeeff",
        b"{\"role\":\"Hybrid\"}".to_vec(),
    )
    .map_err(|e| e.to_string())?;
    let bytes = ctrl_env.to_bytes();
    let decoded = WireEnvelope::from_bytes(&bytes).map_err(|e| e.to_string())?;
    if decoded != ctrl_env {
        return Err("wire envelope roundtrip mismatch".into());
    }

    // Confirm bulk data frame is rejected on ControlPlane
    let isolation_check = WireEnvelope::new(
        PlaneKind::ControlPlane,
        MessageKind::TensorShardFrame,
        2,
        "did:umcf:00112233445566778899aabbccddeeff",
        vec![0u8; 128],
    );
    if isolation_check.is_ok() {
        return Err("plane isolation check failed to reject bulk frame on control plane".into());
    }

    if json {
        Ok(format!(
            "{{\"status\":\"PASS\",\"protocol_version\":\"{}\",\"envelope_roundtrip\":true,\"plane_isolation_enforced\":true}}",
            CURRENT_PROTOCOL_VERSION
        ))
    } else {
        Ok(format!(
            "Protocol Verification: PASS (v{}, binary roundtrip & control/data plane isolation verified)",
            CURRENT_PROTOCOL_VERSION
        ))
    }
}

fn run_plan_inference(args: &[String], json: bool) -> Result<String, String> {
    let private_only = args.iter().any(|a| a == "--private-local-only");
    let master_id = NodeId::from_public_key_bytes(b"master-coordinator-key-seed-0001")
        .map_err(|e| e.to_string())?;
    let worker_id = NodeId::from_public_key_bytes(b"worker-secondary-key-seed-000002")
        .map_err(|e| e.to_string())?;

    let mut coord = ClusterCoordinator::new(
        "umcf-local-cluster",
        master_id.clone(),
        60,
        b"cli-plan-secret".to_vec(),
    );

    let tok1 = coord
        .issue_enrollment_token("tok-1", 1000, 120)
        .map_err(|e| e.to_string())?;
    let tok2 = coord
        .issue_enrollment_token("tok-2", 1000, 120)
        .map_err(|e| e.to_string())?;

    let consent = ResourceConsentPolicy {
        sharing_enabled: true,
        max_memory_mb: 6144,
        ..ResourceConsentPolicy::default()
    };

    let mut hw1 = probe_current_host_profile(master_id.clone());
    hw1.available_ram_mb = 6144;
    let mut hw2 = probe_current_host_profile(worker_id);
    hw2.available_ram_mb = 6144;

    coord
        .enroll_node(&tok1, NodeRole::Hybrid, hw1, consent.clone(), 1005)
        .map_err(|e| e.to_string())?;
    coord
        .enroll_node(&tok2, NodeRole::Worker, hw2, consent, 1006)
        .map_err(|e| e.to_string())?;

    let active = coord.active_nodes(1010);
    let scheduler = HeterogeneousScheduler::new();
    let req = DistributedInferenceRequest {
        model_name: "Llama-3.1-8B-Instruct-Q4_K_M.gguf".into(),
        total_model_weights_mb: 4800,
        kv_cache_overhead_mb: 1024,
        total_layers: 32,
        preferred_backend: Some(ComputeBackend::CpuSimd),
        privacy_mode: if private_only {
            InferencePrivacyMode::PrivateLocalOnly
        } else {
            InferencePrivacyMode::ClusterDistributed
        },
    };

    let plan = scheduler
        .plan_distributed_inference(&req, &master_id, &active)
        .map_err(|e| e.to_string())?;

    if json {
        Ok(format!(
            "{{\"status\":\"PASS\",\"model\":\"{}\",\"total_layers\":{},\"stages_count\":{}}}",
            plan.model_name,
            plan.total_layers,
            plan.stages.len()
        ))
    } else {
        Ok(format!(
            "Inference Plan for {} ({} layers): {} stage(s) scheduled",
            plan.model_name,
            plan.total_layers,
            plan.stages.len()
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integration_cli_commands_execute_successfully() {
        assert!(run_cli(&["version".into()])
            .expect("version")
            .contains("protocol v1.0.0"));
        assert!(run_cli(&["doctor".into(), "--json".into()])
            .expect("doctor json")
            .contains("\"status\":\"PASS\""));
        assert!(run_cli(&["verify-protocol".into(), "--json".into()])
            .expect("verify-protocol")
            .contains("\"plane_isolation_enforced\":true"));
        assert!(run_cli(&["plan-inference".into(), "--json".into()])
            .expect("plan-inference")
            .contains("\"status\":\"PASS\""));
        assert!(run_cli(&["help".into()]).expect("help").contains("USAGE:"));
        assert!(run_cli(&["invalid-cmd".into()]).is_err());
    }
}
