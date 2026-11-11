//! `umcfd` node daemon binary for Universal Mobile Compute Fabric.

use std::env;
use std::process::ExitCode;
use umcf_coordinator::ClusterCoordinator;
use umcf_core::{NodeId, NodeRole, ResourceConsentPolicy, RoleStateMachine};
use umcf_proto::{MessageKind, PlaneKind, WireEnvelope};
use umcf_scheduler::{DistributedInferenceRequest, HeterogeneousScheduler, InferencePrivacyMode};
use umcf_worker::{
    probe_current_host_profile, AdmissionVerdict, LiveDeviceTelemetry, ResourceGuard,
};

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    match run_daemon(&args) {
        Ok(out) => {
            println!("{out}");
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("daemon error: {err}");
            ExitCode::from(1)
        }
    }
}

pub fn run_daemon(args: &[String]) -> Result<String, String> {
    let cmd = args.first().map_or("self-check", String::as_str);
    let json = args.iter().any(|a| a == "--json");

    match cmd {
        "self-check" => execute_self_check(json),
        "print-default-config" => Ok(default_config_toml()),
        "--version" | "version" => Ok(format!("umcfd {}", env!("CARGO_PKG_VERSION"))),
        other => Err(format!("unknown daemon command: {other}")),
    }
}

fn default_config_toml() -> String {
    [
        "[node]",
        "role = \"Hybrid\"",
        "cluster_id = \"umcf-local-mesh\"",
        "",
        "[consent]",
        "sharing_enabled = false",
        "max_cpu_percent = 50",
        "max_memory_mb = 2048",
        "max_storage_mb = 4096",
        "allow_on_battery = false",
        "min_battery_percent = 50",
        "max_temperature_celsius = 75",
        "preempt_on_user_activity = true",
    ]
    .join("\n")
}

fn execute_self_check(json: bool) -> Result<String, String> {
    let node_id = NodeId::from_public_key_bytes(b"umcfd-self-check-node-key-seed-01")
        .map_err(|e| e.to_string())?;
    let mut role_sm = RoleStateMachine::new(node_id.clone(), NodeRole::Worker);
    let epoch = role_sm.transition_to(NodeRole::Hybrid);

    let hw = probe_current_host_profile(node_id.clone());
    let consent = ResourceConsentPolicy {
        sharing_enabled: true,
        max_memory_mb: 4096,
        ..ResourceConsentPolicy::default()
    };

    let guard = ResourceGuard::new(hw.clone(), consent.clone());
    let telemetry = LiveDeviceTelemetry {
        current_cpu_percent: 15,
        allocated_umcf_memory_mb: 256,
        on_battery: false,
        battery_percent: 100,
        temperature_celsius: 48,
        user_currently_active: false,
    };
    if guard.evaluate(1024, &telemetry) != AdmissionVerdict::Admit {
        return Err("resource guard self-check failed to admit valid workload".into());
    }

    let mut coord = ClusterCoordinator::new(
        "umcf-self-check-cluster",
        node_id.clone(),
        60,
        b"daemon-self-check-secret".to_vec(),
    );
    let tok = coord
        .issue_enrollment_token("self-check-token", 2000, 60)
        .map_err(|e| e.to_string())?;
    coord
        .enroll_node(&tok, role_sm.current_role(), hw, consent, 2005)
        .map_err(|e| e.to_string())?;

    let active = coord.active_nodes(2010);
    let scheduler = HeterogeneousScheduler::new();
    let req = DistributedInferenceRequest {
        model_name: "TinyLlama-1.1B-Q4.gguf".into(),
        total_model_weights_mb: 768,
        kv_cache_overhead_mb: 256,
        total_layers: 22,
        preferred_backend: None,
        privacy_mode: InferencePrivacyMode::PrivateLocalOnly,
    };
    let plan = scheduler
        .plan_distributed_inference(&req, &node_id, &active)
        .map_err(|e| e.to_string())?;

    let heartbeat_env = WireEnvelope::new(
        PlaneKind::ControlPlane,
        MessageKind::Heartbeat,
        1,
        node_id.as_str(),
        b"{\"status\":\"healthy\"}".to_vec(),
    )
    .map_err(|e| e.to_string())?;
    let _ = WireEnvelope::from_bytes(&heartbeat_env.to_bytes()).map_err(|e| e.to_string())?;

    if json {
        Ok(format!(
            "{{\"status\":\"PASS\",\"daemon\":\"umcfd\",\"node_id\":\"{}\",\"role\":\"{}\",\"epoch\":{},\"scheduled_stages\":{}}}",
            node_id,
            role_sm.current_role(),
            epoch,
            plan.stages.len()
        ))
    } else {
        Ok(format!(
            "umcfd self-check: PASS (node={}, role={}, epoch={}, stages={})",
            node_id,
            role_sm.current_role(),
            epoch,
            plan.stages.len()
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integration_daemon_self_check_and_config() {
        let out = run_daemon(&["self-check".into(), "--json".into()]).expect("self-check");
        assert!(out.contains("\"status\":\"PASS\""));
        let cfg = run_daemon(&["print-default-config".into()]).expect("config");
        assert!(cfg.contains("sharing_enabled = false"));
        assert!(run_daemon(&["version".into()])
            .expect("ver")
            .contains("umcfd"));
        assert!(run_daemon(&["unknown".into()]).is_err());
    }

    #[test]
    fn integration_daemon_model_registry_service() {
        use tempfile::TempDir;
        use umcf_daemon::ModelRegistryService;
        use umcf_gguf::{
            ContentAddressedModelStore, DownloadScope, MockPeerProvider, CHUNK_SIZE_BYTES,
        };

        let temp = TempDir::new().expect("temp dir");
        let store = ContentAddressedModelStore::new(temp.path()).expect("store");
        let service = ModelRegistryService::new(store);

        let test_file = temp.path().join("model.gguf");
        std::fs::write(&test_file, vec![0x42; (CHUNK_SIZE_BYTES as usize) * 2])
            .expect("write file");

        let manifest = service.import_local_model(&test_file).expect("import");
        assert_eq!(manifest.chunks.len(), 2);

        let fetched = service
            .get_model_manifest(&manifest.model_id)
            .expect("get manifest");
        assert_eq!(fetched.model_id, manifest.model_id);

        let list = service.list_models().expect("list");
        assert_eq!(list.len(), 1);

        let peer = std::sync::Arc::new(
            MockPeerProvider::new("peer-alpha", 2)
                .with_chunk(0, vec![0x42; CHUNK_SIZE_BYTES as usize])
                .with_chunk(1, vec![0x42; CHUNK_SIZE_BYTES as usize]),
        );

        let session_id = service
            .start_model_sync(manifest.clone(), DownloadScope::FullModel, &[peer])
            .expect("start sync");
        assert!(session_id.starts_with("sync-"));

        let progress = service
            .subscribe_sync_progress(&session_id)
            .expect("progress");
        assert_eq!(progress.total_chunks, 2);

        service.pause_model_sync(&session_id).expect("pause");
        service.cancel_model_sync(&session_id).expect("cancel");

        let freed = service
            .evict_model(&manifest.model_id, true)
            .expect("evict");
        assert!(freed > 0);
    }
}
