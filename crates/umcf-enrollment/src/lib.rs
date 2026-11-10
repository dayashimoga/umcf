//! `umcf-enrollment` — Short-lived QR enrollment tokens, matrix rendering, and secure candidate handshake.
//! Strictly enforces `#![forbid(unsafe_code)]` and zero-placeholder invariants.

#![forbid(unsafe_code)]

pub mod consent;
pub mod error;
pub mod handshake;
pub mod qr;
pub mod rbac;
pub mod token;

pub use consent::{
    ConsentEnforcer, ConsentUpdatedEvent, DeviceEnvironmentState, QuietHoursSchedule,
    ResourceConsentPolicy, TaskAssignmentRequest, WorkloadKind, CONSENT_POLICY_DOMAIN,
};
pub use error::EnrollmentError;
pub use handshake::{CandidateEnroller, EnrollmentCoordinator, JoinRequest, ReplayProtectionCache};
pub use qr::QrRenderer;
pub use rbac::{Permission, RbacPolicyEngine, ResourceScope, Role};
pub use token::{EnrollmentToken, DEFAULT_TOKEN_TTL_MS, MAX_TOKEN_TTL_MS, URI_SCHEME_PREFIX};

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::net::SocketAddr;
    use std::path::Path;
    use std::sync::Arc;
    use std::time::Instant;
    use umcf_core::capabilities::{
        CpuCapability, HardwareCapabilityDescriptor, MemoryCapability, NetworkCapability,
        PowerThermalState, StorageCapability,
    };
    use umcf_core::{
        ByteSize, DeviceClass, MegabytesPerSec, NetworkLinkType, NodeId, NodeRole, OperatingSystem,
        ThermalState,
    };
    use umcf_identity::{ClusterCa, NodeKeypair};

    fn test_endpoints() -> Vec<SocketAddr> {
        vec!["192.168.1.100:8443".parse().expect("parse socket addr")]
    }

    fn sample_hardware_caps(node_name: &str) -> HardwareCapabilityDescriptor {
        let node_id = NodeId::parse(format!("did:umcf:{node_name:0<24}")).expect("valid node id");
        HardwareCapabilityDescriptor {
            node_id,
            device_class: DeviceClass::Laptop,
            os: OperatingSystem::Linux,
            cpu: CpuCapability {
                architecture: "x86_64".into(),
                physical_cores: 8,
                performance_cores: 8,
                efficiency_cores: 0,
                simd_avx2: true,
                simd_avx512: false,
                simd_neon: false,
                simd_sve: false,
                base_clock_mhz: 2400,
                boost_clock_mhz: 4800,
            },
            memory: MemoryCapability {
                total_ram: ByteSize::from_gib(16),
                available_ram: ByteSize::from_gib(12),
                is_unified_memory: false,
                bandwidth_mbps: MegabytesPerSec::from_mbps(50_000),
            },
            accelerators: vec![],
            storage: StorageCapability {
                total_disk: ByteSize::from_gib(512),
                available_disk: ByteSize::from_gib(256),
                is_solid_state: true,
            },
            network: NetworkCapability {
                link_type: NetworkLinkType::Wifi,
                estimated_bandwidth_mbps: 500,
                measured_rtt_ms: 12,
            },
            power_thermal: PowerThermalState {
                battery_percent: Some(100),
                is_charging: true,
                thermal_state: ThermalState::Nominal,
                temperature_milli_celsius: Some(45_000),
            },
        }
    }

    #[test]
    fn unit_enrollment_token_generate_uri_roundtrip_ttl_and_qr_rendering() {
        let coordinator = NodeKeypair::generate();
        let cluster_id = "umcf-mesh-beta";

        // 1. Generate token
        let token = EnrollmentToken::generate(
            &coordinator,
            cluster_id,
            [7u8; 32],
            test_endpoints(),
            NodeRole::Worker,
            Some(180_000),
        )
        .expect("generate enrollment token");

        assert_eq!(token.cluster_id, cluster_id);
        assert_eq!(token.issuer_did, coordinator.did_key());
        assert_eq!(token.proposed_role, NodeRole::Worker);
        assert!(token.verify_signature().is_ok());

        // 2. URI serialization & length ceiling (<= 320 ASCII characters)
        let uri = token.to_uri().expect("serialize token to uri");
        assert!(uri.starts_with(URI_SCHEME_PREFIX));
        println!("Enrollment URI length: {} characters", uri.len());
        assert!(
            uri.len() <= 320,
            "URI length {} exceeds 320 ASCII character QR ceiling!",
            uri.len()
        );

        // 3. URI round-trip deserialization
        let recovered = EnrollmentToken::from_uri(&uri).expect("deserialize token from uri");
        assert_eq!(recovered.cluster_id, token.cluster_id);
        assert_eq!(recovered.issuer_did, token.issuer_did);
        assert_eq!(recovered.one_time_nonce, token.one_time_nonce);
        assert_eq!(recovered.expires_at_ms, token.expires_at_ms);
        assert_eq!(recovered.proposed_role, token.proposed_role);
        assert!(recovered.verify_signature().is_ok());

        // 4. TTL expiration verification
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        assert!(!token.is_expired(now_ms));
        assert!(token.is_expired(token.expires_at_ms + 1000));

        // 5. Max TTL ceiling enforcement (> 600s)
        let max_ttl_res = EnrollmentToken::generate(
            &coordinator,
            cluster_id,
            [7u8; 32],
            test_endpoints(),
            NodeRole::Worker,
            Some(601_000),
        );
        assert!(matches!(
            max_ttl_res,
            Err(EnrollmentError::MaxTtlExceeded { .. })
        ));

        // 6. Signature tampering detection
        let mut tampered_token = token.clone();
        tampered_token.proposed_role = NodeRole::Master; // Elevate role
        assert!(tampered_token.verify_signature().is_err());

        // 7. QR rendering: SVG and terminal Unicode blocks
        let svg = QrRenderer::to_svg(&uri).expect("render svg qr");
        assert!(svg.contains("<svg"));
        assert!(svg.contains("</svg>"));

        let unicode_qr = QrRenderer::to_unicode_blocks(&uri).expect("render unicode blocks qr");
        assert!(!unicode_qr.is_empty());
    }

    #[test]
    fn integration_full_master_qr_candidate_scan_and_cert_issuance_handshake() {
        let cluster_id = "umcf-mesh-alpha";
        let coordinator_keypair = Arc::new(NodeKeypair::generate());
        let ca = Arc::new(ClusterCa::generate(cluster_id).expect("generate cluster ca"));

        let coordinator = EnrollmentCoordinator::new(
            cluster_id,
            Arc::clone(&coordinator_keypair),
            Arc::clone(&ca),
        );

        let candidate = NodeKeypair::generate();
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        // 1. Coordinator issues QR token
        let token = coordinator
            .issue_token(test_endpoints(), NodeRole::Worker, Some(180_000))
            .expect("issue token");

        let uri = token.to_uri().expect("to uri");

        // Export sample SVG QR code for verification evidence
        let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        let root_artifacts = manifest_dir
            .parent()
            .and_then(|p| p.parent())
            .map(|p| p.join("artifacts").join("evidence").join("stage-02"))
            .unwrap_or_else(|| std::path::PathBuf::from("artifacts/evidence/stage-02"));

        fs::create_dir_all(&root_artifacts).expect("create evidence dir");
        let svg_path = root_artifacts.join("sample_enrollment_qr.svg");
        QrRenderer::to_svg_file(&uri, &svg_path).expect("write sample enrollment svg");
        assert!(svg_path.exists());

        // Microbenchmark timer for entire handshake:
        // Candidate scans QR URI -> Prepares JoinRequest -> Coordinator processes & issues NodeCertificate
        let start = Instant::now();

        // 2. Candidate scans QR URI
        let scanned_token = EnrollmentToken::from_uri(&uri).expect("candidate scans uri");

        // 3. Candidate creates join request
        let hardware_caps = sample_hardware_caps("candidate-01");
        let join_req = CandidateEnroller::prepare_join_request(
            &candidate,
            &scanned_token,
            hardware_caps,
            now_ms,
        )
        .expect("prepare join request");

        // 4. Coordinator processes join request & issues leaf NodeCertificate
        let node_cert = coordinator
            .process_join_request(&scanned_token, &join_req, now_ms)
            .expect("coordinator processes join request");

        let elapsed = start.elapsed();
        let elapsed_ms = elapsed.as_millis();
        println!("Full enrollment handshake latency: {} ms (target < 50 ms)", elapsed_ms);

        // Verify issued certificate
        assert_eq!(node_cert.did(), candidate.did_key());
        assert_eq!(node_cert.role, NodeRole::Worker);
        assert_eq!(node_cert.cluster_id, cluster_id);
        assert!(!node_cert.serial_number().is_empty());
        assert!(!node_cert.is_expired(now_ms));

        // Acceptance criterion: Handshake completes in < 50 ms over local loopback
        assert!(
            elapsed_ms < 50,
            "Handshake latency {} ms exceeds 50 ms ceiling",
            elapsed_ms
        );
    }

    #[test]
    fn simulation_concurrent_token_replay_rejected_by_cache() {
        let cluster_id = "umcf-mesh-replay-sim";
        let coordinator_keypair = Arc::new(NodeKeypair::generate());
        let ca = Arc::new(ClusterCa::generate(cluster_id).expect("generate cluster ca"));

        let coordinator = EnrollmentCoordinator::new(
            cluster_id,
            Arc::clone(&coordinator_keypair),
            Arc::clone(&ca),
        );

        let token = coordinator
            .issue_token(test_endpoints(), NodeRole::Worker, Some(180_000))
            .expect("issue token");

        let candidate_alice = NodeKeypair::generate();
        let candidate_bob = NodeKeypair::generate();

        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        let hardware_caps = sample_hardware_caps("candidate-test");

        // 1. Candidate Alice prepares and submits join request
        let req_alice = CandidateEnroller::prepare_join_request(
            &candidate_alice,
            &token,
            hardware_caps.clone(),
            now_ms,
        )
        .expect("alice prepare join");

        let cert_alice = coordinator.process_join_request(&token, &req_alice, now_ms);
        assert!(cert_alice.is_ok(), "First redemption by Alice must succeed");

        // 2. Candidate Bob attempts to redeem the same single-use token (Replay Attack)
        let req_bob = CandidateEnroller::prepare_join_request(
            &candidate_bob,
            &token,
            hardware_caps,
            now_ms,
        )
        .expect("bob prepare join");

        let cert_bob = coordinator.process_join_request(&token, &req_bob, now_ms);
        assert!(
            matches!(cert_bob, Err(EnrollmentError::TokenReplayed(_))),
            "Second redemption by Bob must be rejected with TokenReplayed error, got: {:?}",
            cert_bob
        );
    }

    #[test]
    fn unit_rbac_permission_matrix_and_default_deny_microbenchmark() {
        let rbac = RbacPolicyEngine::new();
        let actor = "did:key:z6MkuTestActor1234567890abcdef";
        let scope = ResourceScope::Global;

        // Verify ClusterOwner has all permissions
        for p in [
            Permission::SubmitJob,
            Permission::ScheduleTask,
            Permission::ReadClusterState,
            Permission::EnrollNode,
            Permission::RevokeNode,
            Permission::AccessPrivateAi,
            Permission::StreamDesktop,
            Permission::ReadWriteStorage,
        ] {
            assert!(
                rbac.authorize(actor, Role::ClusterOwner, p, &scope).is_ok(),
                "ClusterOwner must have {p:?}"
            );
        }

        // Verify ObserverGuest only has ReadClusterState
        assert!(rbac
            .authorize(actor, Role::ObserverGuest, Permission::ReadClusterState, &scope)
            .is_ok());
        for p in [
            Permission::SubmitJob,
            Permission::ScheduleTask,
            Permission::EnrollNode,
            Permission::RevokeNode,
            Permission::AccessPrivateAi,
            Permission::StreamDesktop,
            Permission::ReadWriteStorage,
        ] {
            assert!(
                matches!(
                    rbac.authorize(actor, Role::ObserverGuest, p, &scope),
                    Err(EnrollmentError::RbacDenied { .. })
                ),
                "ObserverGuest must NOT have {p:?}"
            );
        }

        // Microbenchmark: Default-deny authorization check latency (< 5 µs target)
        let iterations = 10_000;
        let start = Instant::now();
        for _ in 0..iterations {
            let res = rbac.authorize(actor, Role::ComputeWorker, Permission::EnrollNode, &scope);
            assert!(res.is_err());
        }
        let elapsed = start.elapsed();
        let avg_micros = (elapsed.as_nanos() as f64) / (iterations as f64 * 1000.0);
        println!(
            "RBAC authorization latency: {:.3} µs / op (target < 5.0 µs)",
            avg_micros
        );
        assert!(
            avg_micros < 5.0,
            "RBAC check average {avg_micros} µs exceeds 5 µs ceiling"
        );

        // Export canonical RBAC matrix to artifacts/evidence/stage-02/rbac_consent_matrix.json
        let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        let root_artifacts = manifest_dir
            .parent()
            .and_then(|p| p.parent())
            .map(|p| p.join("artifacts").join("evidence").join("stage-02"))
            .unwrap_or_else(|| std::path::PathBuf::from("artifacts/evidence/stage-02"));

        fs::create_dir_all(&root_artifacts).expect("create evidence dir");
        let matrix_path = root_artifacts.join("rbac_consent_matrix.json");

        let matrix_json = serde_json::json!({
            "subphase": "S2.P2.SP2",
            "title": "UMCF RBAC & Resource-Sharing Consent Matrix",
            "rbac_roles": {
                "ClusterOwner": rbac.get_role_permissions(Role::ClusterOwner).iter().map(|p| p.to_string()).collect::<Vec<_>>(),
                "MasterCoordinator": rbac.get_role_permissions(Role::MasterCoordinator).iter().map(|p| p.to_string()).collect::<Vec<_>>(),
                "HybridNode": rbac.get_role_permissions(Role::HybridNode).iter().map(|p| p.to_string()).collect::<Vec<_>>(),
                "ComputeWorker": rbac.get_role_permissions(Role::ComputeWorker).iter().map(|p| p.to_string()).collect::<Vec<_>>(),
                "StorageProvider": rbac.get_role_permissions(Role::StorageProvider).iter().map(|p| p.to_string()).collect::<Vec<_>>(),
                "ObserverGuest": rbac.get_role_permissions(Role::ObserverGuest).iter().map(|p| p.to_string()).collect::<Vec<_>>()
            },
            "default_deny_enforced": true,
            "measured_avg_latency_micros": avg_micros
        });

        fs::write(
            &matrix_path,
            serde_json::to_string_pretty(&matrix_json).expect("serialize matrix json"),
        )
        .expect("write rbac matrix file");
        assert!(matrix_path.exists());
    }

    #[test]
    fn integration_worker_consent_enforcer_rejects_gpu_workload_and_immediate_policy_update() {
        let worker_keypair = NodeKeypair::generate();
        let worker_node_id =
            NodeId::parse("did:umcf:000000000000worker00001").expect("valid node id");

        let rbac = Arc::new(RbacPolicyEngine::new());

        // Worker policy initially allows ONLY CpuBatch
        let mut initial_allowed = std::collections::HashSet::new();
        initial_allowed.insert(WorkloadKind::CpuBatch);

        let initial_policy = ResourceConsentPolicy::sign_new(
            &worker_keypair,
            worker_node_id.clone(),
            80,
            ByteSize::from_gib(16),
            ByteSize::from_gib(8),
            ByteSize::from_gib(100),
            initial_allowed,
            true, // require charging
            20,   // min battery 20%
            None,
            vec![], // open to cluster
            1,      // generation 1
            1_000_000,
        )
        .expect("sign initial policy");

        let enforcer = ConsentEnforcer::new(
            initial_policy,
            worker_keypair.ed25519_public_bytes(),
            Arc::clone(&rbac),
        )
        .expect("create enforcer");

        let master_keypair = NodeKeypair::generate();
        let current_env = DeviceEnvironmentState {
            battery_percent: Some(85),
            is_charging: true,
            thermal_state: ThermalState::Nominal,
            current_epoch_ms: 1_000_100,
            current_hour_utc: 14,
        };

        // 1. Master attempts to schedule GpuInference on Worker
        let gpu_request = TaskAssignmentRequest {
            requester_did: master_keypair.did_key(),
            requester_role: Role::MasterCoordinator,
            workload_kind: WorkloadKind::GpuInference,
            requested_cpu_cores: 4.0,
            requested_ram: ByteSize::from_gib(4),
            requested_vram: ByteSize::from_gib(2),
            requested_storage: ByteSize::from_gib(1),
        };

        let reject_res = enforcer.evaluate_task_assignment(&gpu_request, &current_env);
        assert!(
            matches!(reject_res, Err(EnrollmentError::WorkloadNotPermitted { .. })),
            "Expected WorkloadNotPermitted error for GPU inference, got: {:?}",
            reject_res
        );

        // 2. Worker owner updates policy to also allow GpuInference
        let update_start = Instant::now();
        let mut updated_allowed = std::collections::HashSet::new();
        updated_allowed.insert(WorkloadKind::CpuBatch);
        updated_allowed.insert(WorkloadKind::GpuInference);

        let updated_policy = ResourceConsentPolicy::sign_new(
            &worker_keypair,
            worker_node_id,
            90,
            ByteSize::from_gib(16),
            ByteSize::from_gib(8),
            ByteSize::from_gib(100),
            updated_allowed,
            true,
            20,
            None,
            vec![],
            2, // generation 2
            1_000_200,
        )
        .expect("sign updated policy");

        let update_event = enforcer.update_policy(updated_policy).expect("update policy");
        let update_elapsed = update_start.elapsed();
        let update_ms = update_elapsed.as_millis();
        println!("Policy update latency: {} ms (target < 10 ms)", update_ms);

        // Acceptance criterion: Policy updates take effect immediately in < 10 ms
        assert!(
            update_ms < 10,
            "Policy update elapsed {} ms exceeds 10 ms ceiling",
            update_ms
        );
        assert_eq!(update_event.policy_generation, 2);

        // 3. Master attempts GPU task again -> Now succeeds!
        let accept_res = enforcer.evaluate_task_assignment(&gpu_request, &current_env);
        assert!(
            accept_res.is_ok(),
            "Expected task assignment to succeed after consent update"
        );
    }

    #[test]
    fn simulation_forged_consent_policy_by_foreign_master_rejected() {
        let worker_keypair = NodeKeypair::generate();
        let worker_node_id =
            NodeId::parse("did:umcf:000000000000worker00002").expect("valid node id");

        let rbac = Arc::new(RbacPolicyEngine::new());
        let mut initial_allowed = std::collections::HashSet::new();
        initial_allowed.insert(WorkloadKind::CpuBatch);

        let legitimate_policy = ResourceConsentPolicy::sign_new(
            &worker_keypair,
            worker_node_id.clone(),
            50,
            ByteSize::from_gib(8),
            ByteSize::ZERO,
            ByteSize::from_gib(10),
            initial_allowed,
            true,
            50,
            None,
            vec![],
            1,
            1_000_000,
        )
        .expect("sign legitimate policy");

        let enforcer = ConsentEnforcer::new(
            legitimate_policy,
            worker_keypair.ed25519_public_bytes(),
            Arc::clone(&rbac),
        )
        .expect("create enforcer");

        // Adversarial Master node attempts to forge an update to override local sovereignty
        let foreign_master_keypair = NodeKeypair::generate();
        let mut forged_allowed = std::collections::HashSet::new();
        forged_allowed.insert(WorkloadKind::CpuBatch);
        forged_allowed.insert(WorkloadKind::GpuInference);
        forged_allowed.insert(WorkloadKind::GpuRendering);

        // Foreign master signs with its own keypair claiming to represent the worker
        let forged_policy = ResourceConsentPolicy::sign_new(
            &foreign_master_keypair, // FORGED SIGNER
            worker_node_id,
            100, // force 100% CPU
            ByteSize::from_gib(64),
            ByteSize::from_gib(32),
            ByteSize::from_gib(1000),
            forged_allowed,
            false, // remove AC power requirement
            0,     // remove battery floor
            None,
            vec![],
            2,
            1_000_500,
        )
        .expect("sign forged policy");

        // Worker enforcer must strictly reject this foreign-signed update
        let res = enforcer.update_policy(forged_policy);
        assert!(
            matches!(res, Err(EnrollmentError::ForgedConsentPolicy { .. })),
            "Expected ForgedConsentPolicy error for foreign master signature, got: {:?}",
            res
        );
    }
}
