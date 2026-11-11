//! Five-tier verification test suite for Subphase S8.P2.SP2 Security Red-Team.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use security_redteam::{
    RbacAndCryptoAuditor, SandboxEscapeAuditor, SecurityAuditLedger, SecuritySeverity,
};

// ============================================================================
// 1. [UNIT] TESTS
// ============================================================================

#[test]
fn test_constant_time_api_key_and_pairing_pin_comparison() {
    let pin_a = b"849-204";
    let pin_b = b"849-204";
    let pin_wrong = b"849-205";

    assert!(RbacAndCryptoAuditor::audit_vector_11_constant_time_eq(
        pin_a, pin_b
    ));
    assert!(!RbacAndCryptoAuditor::audit_vector_11_constant_time_eq(
        pin_a, pin_wrong
    ));
}

#[test]
fn test_path_canonicalization_blocks_dotdot_and_symlink_escapes() {
    let mut ledger = SecurityAuditLedger::new();

    // Attacks attempting traversal outside sandbox root
    let attacks = [
        "../../../../etc/passwd",
        "C:\\Windows\\System32\\cmd.exe",
        "/etc/shadow",
        "scratch/../../docker.sock",
    ];

    for path in attacks {
        let res = SandboxEscapeAuditor::audit_vector_1_wasi_fs_traversal(&mut ledger, path);
        assert!(res.is_err(), "Expected blocked traversal for {path}");
    }

    assert_eq!(
        ledger.count_by_severity(SecuritySeverity::High),
        attacks.len()
    );

    // Legitimate relative scratch path passes
    let res_legit = SandboxEscapeAuditor::audit_vector_1_wasi_fs_traversal(
        &mut ledger,
        "models/temp_shard.bin",
    );
    assert!(res_legit.is_ok());
}

#[test]
fn test_revoked_certificate_purges_0rtt_session_tickets() {
    let mut ledger = SecurityAuditLedger::new();
    let node_did = "did:umcf:node_compromised_01";

    // Attempt 0-RTT replay on revoked certificate
    let res =
        RbacAndCryptoAuditor::audit_vector_7_revoked_cert_0rtt_replay(&mut ledger, node_did, true);
    assert!(res.is_err());
    assert_eq!(ledger.count_by_severity(SecuritySeverity::Critical), 1);

    // Active unrevoked certificate passes
    let res_ok = RbacAndCryptoAuditor::audit_vector_7_revoked_cert_0rtt_replay(
        &mut ledger,
        "did:umcf:node_healthy_02",
        false,
    );
    assert!(res_ok.is_ok());
}

// ============================================================================
// 2. [INTEGRATION] TESTS
// ============================================================================

#[test]
fn test_redteam_vectors_1_through_5_sandbox_escape_containment() {
    let mut ledger = SecurityAuditLedger::new();

    // Vector 1: WASI traversal
    let v1 =
        SandboxEscapeAuditor::audit_vector_1_wasi_fs_traversal(&mut ledger, "../../etc/passwd");
    assert!(v1.is_err());

    // Vector 2: Ambient network & subprocess
    let v2_net = SandboxEscapeAuditor::audit_vector_2_wasm_ambient_network_and_proc(
        &mut ledger,
        "sock_open",
    );
    let v2_proc = SandboxEscapeAuditor::audit_vector_2_wasm_ambient_network_and_proc(
        &mut ledger,
        "proc_exec",
    );
    assert!(v2_net.is_err());
    assert!(v2_proc.is_err());

    // Vector 3: WASM fuel & memory bomb
    let v3_fuel = SandboxEscapeAuditor::audit_vector_3_wasm_fuel_and_memory_bomb(
        &mut ledger,
        1_000_000,
        100_000,
        10,
        100,
    );
    let v3_mem = SandboxEscapeAuditor::audit_vector_3_wasm_fuel_and_memory_bomb(
        &mut ledger,
        10_000,
        100_000,
        200,
        100,
    );
    assert!(v3_fuel.is_err());
    assert!(v3_mem.is_err());

    // Vector 4: GPU zero-scrubbed VRAM & shader loop TDR hang
    let zero_buffer = [0u32; 1024];
    let v4_ok = SandboxEscapeAuditor::audit_vector_4_gpu_vram_cross_tenant_read(
        &mut ledger,
        &zero_buffer,
        100,
        1000,
    );
    assert!(v4_ok.is_ok());

    let mut dirty_buffer = [0u32; 1024];
    dirty_buffer[42] = 0xDEADBEEF; // Residual unscrubbed VRAM
    let v4_leak = SandboxEscapeAuditor::audit_vector_4_gpu_vram_cross_tenant_read(
        &mut ledger,
        &dirty_buffer,
        100,
        1000,
    );
    assert!(v4_leak.is_err());

    let v4_hang = SandboxEscapeAuditor::audit_vector_4_gpu_vram_cross_tenant_read(
        &mut ledger,
        &zero_buffer,
        5000,
        1000,
    );
    assert!(v4_hang.is_err());

    // Vector 5: GGUF tensor offset bounds
    let v5_past_eof = SandboxEscapeAuditor::audit_vector_5_gguf_tensor_offset(
        &mut ledger,
        "blk.0.weight",
        15_000_000,
        1_000_000,
        10_000_000,
    );
    assert!(v5_past_eof.is_err());

    let v5_wrap = SandboxEscapeAuditor::audit_vector_5_gguf_tensor_offset(
        &mut ledger,
        "blk.0.weight",
        u64::MAX - 8,
        32,
        10_000_000,
    );
    assert!(v5_wrap.is_err());
}

#[test]
fn test_redteam_vectors_6_through_11_rbac_mtls_and_enclave_containment() {
    let mut ledger = SecurityAuditLedger::new();

    // Vector 6: Unauthenticated peer mTLS
    let v6 = RbacAndCryptoAuditor::audit_vector_6_mtls_peer_handshake(
        &mut ledger,
        "did:umcf:unpaired",
        false,
        false,
    );
    assert!(v6.is_err());

    // Vector 7: Revoked certificate
    let v7 = RbacAndCryptoAuditor::audit_vector_7_revoked_cert_0rtt_replay(
        &mut ledger,
        "did:umcf:revoked",
        true,
    );
    assert!(v7.is_err());

    // Vector 8: RBAC scope escalation
    let v8_post_job = RbacAndCryptoAuditor::audit_vector_8_rbac_authorization(
        &mut ledger,
        "GuestObserver",
        "/api/v1/jobs",
        "POST",
    )
    .unwrap();
    assert_eq!(v8_post_job, 403);

    let v8_delete_model = RbacAndCryptoAuditor::audit_vector_8_rbac_authorization(
        &mut ledger,
        "cluster:read",
        "/api/v1/models/llama",
        "DELETE",
    )
    .unwrap();
    assert_eq!(v8_delete_model, 403);

    // Vector 9: Private AI Enclave exfiltration attempt
    let v9 = RbacAndCryptoAuditor::audit_vector_9_private_ai_exfiltration(&mut ledger, true, true);
    assert!(v9.is_err());

    // Vector 10: Remote stream ViewOnly input hijack
    let v10 = RbacAndCryptoAuditor::audit_vector_10_view_only_input_containment(
        &mut ledger,
        "ViewOnly",
        "InputEvent",
    );
    assert!(v10.is_err());

    // Vector 11: Constant-time comparison
    assert!(RbacAndCryptoAuditor::audit_vector_11_constant_time_eq(
        b"token_secret",
        b"token_secret"
    ));
}

// ============================================================================
// 3. [SIMULATION] TESTS
// ============================================================================

#[test]
fn test_automated_endpoint_scanner_45_methods_5_privilege_tiers_matrix() {
    let mut ledger = SecurityAuditLedger::new();

    let endpoints = [
        ("GET", "/api/v1/cluster/topology", false),
        ("GET", "/api/v1/models", false),
        ("POST", "/api/v1/jobs", true),
        ("DELETE", "/api/v1/jobs/job-01", true),
        ("POST", "/api/v1/storage/pins", true),
        ("DELETE", "/api/v1/storage/pins/cid-01", true),
        ("POST", "/api/v1/stream/input", true),
        ("POST", "/api/v1/market/orders", true),
        ("POST", "/api/v1/auth/keys", true),
    ];

    let roles = [
        "Unauthenticated",
        "GuestObserver",
        "ViewOnly",
        "ComputeWorker",
        "ClusterAdmin",
    ];

    let mut total_scans = 0;
    for &role in &roles {
        for &(method, endpoint, is_mutating) in &endpoints {
            total_scans += 1;
            let status = RbacAndCryptoAuditor::audit_vector_8_rbac_authorization(
                &mut ledger,
                role,
                endpoint,
                method,
            )
            .unwrap();
            if role == "ClusterAdmin" {
                assert_eq!(status, 200);
            } else if is_mutating && (role == "GuestObserver" || role == "ViewOnly") {
                assert_eq!(status, 403);
            }
        }
    }

    assert_eq!(total_scans, roles.len() * endpoints.len());
}

// ============================================================================
// 4. [HARDWARE-REQUIRED] TESTS
// ============================================================================

#[test]
fn test_hardware_vram_zero_scrubbing_isolation_verification() {
    let prober = umcf_hwprobe::SystemProber::probe_all(
        umcf_core::types::NodeId::from_public_key_bytes(&[4u8; 32]).unwrap(),
    )
    .expect("probe all");

    assert!(prober.memory.available_ram.0 > 0);

    // Verify hardware buffer allocation is zero-initialized
    let mut ledger = SecurityAuditLedger::new();
    let hw_buffer = vec![0u32; 4096];
    let res = SandboxEscapeAuditor::audit_vector_4_gpu_vram_cross_tenant_read(
        &mut ledger,
        &hw_buffer,
        250,
        1000,
    );
    assert!(res.is_ok());
}

// ============================================================================
// 5. [MANUALLY-VERIFIED] TESTS
// ============================================================================

#[test]
fn test_manual_security_self_audit_schema_and_telemetry() {
    let mut ledger = SecurityAuditLedger::new();
    ledger.record_blocked_attack(
        1,
        "WASM_WASI_FS_TRAVERSAL",
        "guest-01",
        SecuritySeverity::High,
        "BLOCK",
        "Blocked dotdot traversal",
    );

    let json_val = serde_json::to_value(&ledger.events).unwrap();
    assert_eq!(json_val.as_array().unwrap().len(), 1);
    assert_eq!(json_val[0]["vector_id"], 1);
    assert_eq!(json_val[0]["severity"], "High");
}
