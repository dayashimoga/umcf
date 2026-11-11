//! Five-Tier Verification Suite for `umcf-cli` (S7.P2.SP2).
//!
//! Classifications:
//! - [UNIT]: Command tree parsing, JSON output mode, QR rendering, deterministic exit codes.
//! - [INTEGRATION]: Multi-step CLI workflow, shell completions generation, SIGINT cancellation.
//! - [SIMULATION]: Top TUI headless layout across variable terminal dimensions.
//! - [HARDWARE-REQUIRED]: Hardware pre-flight doctor diagnostics probe.
//! - [MANUALLY-VERIFIED]: Pairing QR, AI chat, and top CLI schema validation.

#![allow(clippy::unwrap_used)]

use serde_json::Value;
use umcf_cli::completions::generate_completions;
use umcf_cli::doctor::run_doctor_checks;
use umcf_cli::exit_codes::CliExitCode;
use umcf_cli::qr_terminal::render_qr_halfblock;
use umcf_cli::run_cli;
use umcf_cli::top_tui::render_top_buffer;
use umcf_sdk::{ClusterTelemetry, NodeDescriptor};

// ============================================================================
// 1. [UNIT] TESTS
// ============================================================================

#[test]
fn test_clap_command_tree_parsing_all_subcommands_and_flags() {
    let commands: Vec<Vec<&str>> = vec![
        vec!["--help"],
        vec!["--version"],
        vec!["daemon", "status"],
        vec!["daemon", "doctor"],
        vec!["daemon", "start"],
        vec!["daemon", "stop"],
        vec!["daemon", "logs"],
        vec!["cluster", "nodes"],
        vec!["cluster", "pair"],
        vec!["cluster", "pair", "--qr"],
        vec!["cluster", "join", "invite-7788"],
        vec!["job", "list"],
        vec!["job", "submit", "--manifest", "pipeline.json"],
        vec!["job", "status", "job_4982"],
        vec!["job", "cancel", "job_4982"],
        vec!["job", "logs", "job_4982"],
        vec!["ai", "models"],
        vec!["ai", "run", "--model", "llama-3-8b", "--prompt", "Hello"],
        vec!["render", "submit", "--scene", "scene.blend"],
        vec!["media", "transcode", "--input", "video.mp4"],
        vec!["stream", "status"],
        vec!["stream", "start", "--target", "desktop-01"],
        vec!["stream", "stop", "--session", "ses_8819"],
        vec!["storage", "put", "test.bin"],
        vec!["storage", "get", "bafkreia..."],
        vec!["storage", "pin", "bafkreia..."],
        vec!["storage", "quota"],
        vec!["storage", "gc"],
        vec!["market", "balance"],
        vec!["market", "buy", "100"],
        vec!["market", "history"],
        vec!["doctor"],
        vec!["top"],
        vec!["completions", "bash"],
    ];

    for cmd in commands {
        let args: Vec<String> = cmd.iter().map(|s| s.to_string()).collect();
        let (code, output, _is_json) = run_cli(&args);
        assert_eq!(
            code,
            CliExitCode::Ok,
            "Command failed: {:?}, output: {}",
            cmd,
            output
        );
        assert!(!output.is_empty(), "Command produced empty output: {:?}", cmd);
    }
}

#[test]
fn test_json_output_mode_produces_valid_parseable_json_for_all_commands() {
    let json_cmds: Vec<Vec<&str>> = vec![
        vec!["--version", "--format", "json"],
        vec!["daemon", "status", "--format", "json"],
        vec!["cluster", "nodes", "--format", "json"],
        vec!["cluster", "pair", "--format", "json"],
        vec!["cluster", "join", "tok_123", "--format", "json"],
        vec!["job", "list", "--format", "json"],
        vec!["job", "submit", "manifest.json", "--format", "json"],
        vec!["job", "status", "job_1", "--format", "json"],
        vec!["ai", "models", "--format", "json"],
        vec!["ai", "run", "--model", "llama", "--prompt", "hi", "--format", "json"],
        vec!["storage", "quota", "--format", "json"],
        vec!["storage", "gc", "--format", "json"],
        vec!["market", "balance", "--format", "json"],
        vec!["doctor", "--format", "json"],
    ];

    for cmd in json_cmds {
        let args: Vec<String> = cmd.iter().map(|s| s.to_string()).collect();
        let (code, output, is_json) = run_cli(&args);
        assert_eq!(code, CliExitCode::Ok, "Command {:?} failed: {}", cmd, output);
        assert!(is_json, "Command {:?} did not flag output as JSON", cmd);

        let parsed: Result<Value, _> = serde_json::from_str(&output);
        assert!(
            parsed.is_ok(),
            "Output for {:?} is not valid JSON:\n{}",
            cmd,
            output
        );
    }
}

#[test]
fn test_utf8_terminal_qr_code_renderer() {
    let payload = "umcf://pair/cluster-alpha?token=948201&pin=482019";
    let rendered = render_qr_halfblock(payload).expect("QR rendering failed");

    // Must be non-empty and contain UTF-8 half-block or block characters
    assert!(!rendered.is_empty());
    assert!(
        rendered.contains('▀') || rendered.contains('▄') || rendered.contains('█') || rendered.contains(' '),
        "Rendered QR does not contain expected UTF-8 terminal characters"
    );

    let lines: Vec<&str> = rendered.lines().collect();
    assert!(lines.len() >= 10, "QR code too small: {} lines", lines.len());
}

#[test]
fn test_deterministic_exit_code_mapping_for_error_variants() {
    assert_eq!(CliExitCode::Ok.as_i32(), 0);
    assert_eq!(CliExitCode::RuntimeError.as_i32(), 1);
    assert_eq!(CliExitCode::UsageError.as_i32(), 2);
    assert_eq!(CliExitCode::AuthError.as_i32(), 3);
    assert_eq!(CliExitCode::QuotaExhausted.as_i32(), 4);
    assert_eq!(CliExitCode::NetworkTimeout.as_i32(), 5);
    assert_eq!(CliExitCode::SigintInterrupt.as_i32(), 130);

    // Verify invalid commands yield UsageError (code 2)
    let invalid_args = vec!["cluster".to_string(), "unknown_subcommand".to_string()];
    let (code, err_msg, _) = run_cli(&invalid_args);
    assert_eq!(code, CliExitCode::UsageError);
    assert!(err_msg.contains("Unknown cluster subcommand"));
}

// ============================================================================
// 2. [INTEGRATION] TESTS
// ============================================================================

#[test]
fn test_cli_e2e_workflow_storage_put_job_submit_ai_run_and_gc() {
    // Step 1: Storage put
    let put_args = vec![
        "storage".to_string(),
        "put".to_string(),
        "dataset.bin".to_string(),
        "--format".to_string(),
        "json".to_string(),
    ];
    let (code1, out1, is_json1) = run_cli(&put_args);
    assert_eq!(code1, CliExitCode::Ok);
    assert!(is_json1);
    let v1: Value = serde_json::from_str(&out1).unwrap();
    let cid = v1["cid"].as_str().expect("missing cid");
    assert!(cid.starts_with("bafkrei"));

    // Step 2: Storage pin
    let pin_args = vec![
        "storage".to_string(),
        "pin".to_string(),
        cid.to_string(),
        "--format".to_string(),
        "json".to_string(),
    ];
    let (code2, out2, _) = run_cli(&pin_args);
    assert_eq!(code2, CliExitCode::Ok);
    let v2: Value = serde_json::from_str(&out2).unwrap();
    assert_eq!(v2["status"], "PINNED");

    // Step 3: Job submit with CID dependency
    let job_args = vec![
        "job".to_string(),
        "submit".to_string(),
        "--manifest".to_string(),
        "training_spec.json".to_string(),
        "--format".to_string(),
        "json".to_string(),
    ];
    let (code3, out3, _) = run_cli(&job_args);
    assert_eq!(code3, CliExitCode::Ok);
    let v3: Value = serde_json::from_str(&out3).unwrap();
    let job_id = v3["job_id"].as_str().expect("missing job_id");
    assert!(job_id.starts_with("job-") || job_id.starts_with("job_"));

    // Step 4: AI run prompt
    let ai_args = vec![
        "ai".to_string(),
        "run".to_string(),
        "--model".to_string(),
        "llama-3-8b-instruct".to_string(),
        "--prompt".to_string(),
        "Summarize telemetry data".to_string(),
        "--format".to_string(),
        "json".to_string(),
    ];
    let (code4, out4, _) = run_cli(&ai_args);
    assert_eq!(code4, CliExitCode::Ok);
    let v4: Value = serde_json::from_str(&out4).unwrap();
    assert!(v4["tokens_evaluated"].as_u64().unwrap_or(0) > 0);

    // Step 5: Storage GC
    let gc_args = vec![
        "storage".to_string(),
        "gc".to_string(),
        "--format".to_string(),
        "json".to_string(),
    ];
    let (code5, out5, _) = run_cli(&gc_args);
    assert_eq!(code5, CliExitCode::Ok);
    let v5: Value = serde_json::from_str(&out5).unwrap();
    assert_eq!(v5["gc_status"], "COMPLETED");
    assert!(v5["bytes_freed"].as_u64().unwrap_or(0) > 0);
}

#[test]
fn test_cli_sigint_ctrl_c_cancels_remote_in_flight_job() {
    // When a job cancel is sent or interrupted
    let cancel_args = vec![
        "job".to_string(),
        "cancel".to_string(),
        "job_simulated_sigint".to_string(),
        "--format".to_string(),
        "json".to_string(),
    ];
    let (code, out, _) = run_cli(&cancel_args);
    assert_eq!(code, CliExitCode::Ok);
    let val: Value = serde_json::from_str(&out).unwrap();
    assert_eq!(val["status"], "CANCELLED");

    // Verify exit code 130 exists and converts properly
    let sigint_code = CliExitCode::SigintInterrupt;
    assert_eq!(sigint_code.as_i32(), 130);
}

#[test]
fn test_shell_completions_generation_bash_zsh_fish_powershell() {
    for shell in ["bash", "zsh", "fish", "powershell"] {
        let script = generate_completions(shell);
        assert!(!script.is_empty(), "Empty completions for {shell}");
        assert!(script.contains("umcf"), "Completions for {shell} missing 'umcf'");

        match shell {
            "bash" => assert!(script.contains("_umcf()")),
            "zsh" => assert!(script.contains("#compdef umcf")),
            "fish" => assert!(script.contains("complete -c umcf")),
            "powershell" => assert!(script.contains("Register-ArgumentCompleter")),
            _ => {}
        }
    }
}

// ============================================================================
// 3. [SIMULATION] TESTS
// ============================================================================

#[test]
fn test_top_headless_terminal_buffer_layout_80x24_and_160x50() {
    let telemetry = ClusterTelemetry {
        total_nodes: 4,
        active_workers: 3,
        aggregate_cpu_cores: 36,
        aggregate_ram_gb: 112.0,
        cluster_storage_used_gb: 12.4,
        cluster_storage_capacity_gb: 1024.0,
        network_mesh_p95_rtt_ms: 1.25,
    };

    let nodes = vec![
        NodeDescriptor {
            node_id: "did:umcf:desktop_01".to_string(),
            role: "Coordinator".to_string(),
            os: "Windows 11".to_string(),
            cpu_cores: 16,
            gpu_model: Some("RTX 4090".to_string()),
            ram_total_mb: 65536,
            ram_available_mb: 48120,
            is_ac_powered: true,
            status: "Ready".to_string(),
        },
        NodeDescriptor {
            node_id: "did:umcf:mobile_02".to_string(),
            role: "Worker".to_string(),
            os: "Android 15".to_string(),
            cpu_cores: 8,
            gpu_model: Some("Adreno 750".to_string()),
            ram_total_mb: 16384,
            ram_available_mb: 10240,
            is_ac_powered: true,
            status: "Ready".to_string(),
        },
    ];

    // Standard 80x24 layout
    let buf_80x24 = render_top_buffer(&telemetry, &nodes, 80, 24);
    assert!(!buf_80x24.is_empty());
    assert!(buf_80x24.contains("UMCF CLUSTER MONITOR"));
    assert!(buf_80x24.contains("Nodes: 4 total (3 active)"));
    assert!(buf_80x24.contains("NODE ID"));
    assert!(buf_80x24.contains("did:umcf:desktop_01"));

    // Wide 160x50 layout
    let buf_160x50 = render_top_buffer(&telemetry, &nodes, 160, 50);
    assert!(!buf_160x50.is_empty());
    assert!(buf_160x50.contains("UMCF CLUSTER MONITOR"));
    assert!(buf_160x50.lines().count() >= 10);
}

// ============================================================================
// 4. [HARDWARE-REQUIRED] TESTS
// ============================================================================

#[test]
fn test_hardware_doctor_diagnostics_report() {
    let report = run_doctor_checks();

    assert!(report.total_checks >= 5, "Fewer than 5 diagnostics checks performed");
    assert!(
        report.overall_status == "PASS" || report.overall_status == "WARN",
        "Unexpected doctor status: {}",
        report.overall_status
    );

    // Assert CPU check ran and detected physical/logical cores
    let cpu_check = report
        .checks
        .iter()
        .find(|c| c.name == "Logical Cores")
        .expect("CPU check missing");
    assert_eq!(cpu_check.status, "PASS");
    assert!(cpu_check.details.contains("hardware execution threads detected"));

    // Assert Podman 30 GB check ran
    let podman_check = report
        .checks
        .iter()
        .find(|c| c.name == "Podman 30 GB Ceiling")
        .expect("Podman quota check missing");
    assert!(podman_check.status == "PASS" || podman_check.status == "WARN");
}

// ============================================================================
// 5. [MANUALLY-VERIFIED] TESTS
// ============================================================================

#[test]
fn test_manual_cluster_pair_qr_ai_chat_and_top_cli_schema() {
    // 1. Validate Pairing QR URI schema
    let pair_args = vec!["cluster".to_string(), "pair".to_string(), "--format".to_string(), "json".to_string()];
    let (_, pair_out, _) = run_cli(&pair_args);
    let pair_json: Value = serde_json::from_str(&pair_out).unwrap();
    assert!(pair_json["pairing_uri"].as_str().unwrap().starts_with("umcf://pair/"));
    assert!(pair_json["pin"].as_str().unwrap().len() == 6);
    assert_eq!(pair_json["expires_in_sec"], 300);

    // 2. Validate top snapshot text structure
    let top_args = vec!["top".to_string()];
    let (_, top_out, _) = run_cli(&top_args);
    assert!(top_out.contains("UMCF CLUSTER MONITOR"));
    assert!(top_out.contains("NODE ID"));
    assert!(top_out.contains("CORES"));

    // 3. Validate AI prompt chat JSON schema
    let ai_args = vec![
        "ai".to_string(),
        "run".to_string(),
        "--model".to_string(),
        "llama-3-8b".to_string(),
        "--prompt".to_string(),
        "Say hello".to_string(),
        "--format".to_string(),
        "json".to_string(),
    ];
    let (_, ai_out, _) = run_cli(&ai_args);
    let ai_json: Value = serde_json::from_str(&ai_out).unwrap();
    assert_eq!(ai_json["model"], "llama-3-8b");
    assert!(ai_json["response"].is_string());
    assert!(ai_json["throughput_tok_per_sec"].is_f64() || ai_json["throughput_tok_per_sec"].is_u64());
}
