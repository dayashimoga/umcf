//! `umcf-enrollment` — Short-lived QR enrollment tokens, matrix rendering, and secure candidate handshake.
//! Strictly enforces `#![forbid(unsafe_code)]` and zero-placeholder invariants.

#![forbid(unsafe_code)]

pub mod consent;
pub mod election;
pub mod error;
pub mod handshake;
pub mod qr;
pub mod rbac;
pub mod role_engine;
pub mod token;

pub use consent::{
    ConsentEnforcer, ConsentUpdatedEvent, DeviceEnvironmentState, QuietHoursSchedule,
    ResourceConsentPolicy, TaskAssignmentRequest, WorkloadKind, CONSENT_POLICY_DOMAIN,
};
pub use election::{
    ElectionCoordinator, LeaderHeartbeat, LeaderTransfer, MasterFitnessScore, NodeFitnessProfile,
    RequestVote, VoteResponse,
};
pub use error::EnrollmentError;
pub use handshake::{CandidateEnroller, EnrollmentCoordinator, JoinRequest, ReplayProtectionCache};
pub use qr::QrRenderer;
pub use rbac::{Permission, RbacPolicyEngine, ResourceScope, Role};
pub use role_engine::{NodeRoleState, RoleStateMachine, RoleTransitionEvent};
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
        println!(
            "Full enrollment handshake latency: {} ms (target < 50 ms)",
            elapsed_ms
        );

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
        let req_bob =
            CandidateEnroller::prepare_join_request(&candidate_bob, &token, hardware_caps, now_ms)
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
            .authorize(
                actor,
                Role::ObserverGuest,
                Permission::ReadClusterState,
                &scope
            )
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
            matches!(
                reject_res,
                Err(EnrollmentError::WorkloadNotPermitted { .. })
            ),
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

        let update_event = enforcer
            .update_policy(updated_policy)
            .expect("update policy");
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

    #[test]
    fn unit_master_fitness_score_ranking_across_heterogeneous_profiles() {
        let term = 1;

        // 1. Dedicated plugged-in Server (highest expected fitness)
        let server = NodeFitnessProfile {
            did: "did:key:z6MkServer000000000000000000000001".to_string(),
            device_class: DeviceClass::Server,
            is_charging: true,
            battery_percent: None,
            available_ram: ByteSize::from_gib(32),
            packet_loss_rate: 0.0,
        };
        let score_server = MasterFitnessScore::calculate(&server, term);

        // 2. Plugged-in Laptop (moderate expected fitness)
        let plugged_laptop = NodeFitnessProfile {
            did: "did:key:z6MkPluggedLaptop00000000000000002".to_string(),
            device_class: DeviceClass::Laptop,
            is_charging: true,
            battery_percent: Some(100),
            available_ram: ByteSize::from_gib(16),
            packet_loss_rate: 0.0,
        };
        let score_laptop = MasterFitnessScore::calculate(&plugged_laptop, term);

        // 3. Discharging Laptop (lower fitness due to battery state)
        let battery_laptop = NodeFitnessProfile {
            did: "did:key:z6MkBattLaptop0000000000000000003".to_string(),
            device_class: DeviceClass::Laptop,
            is_charging: false,
            battery_percent: Some(75),
            available_ram: ByteSize::from_gib(8),
            packet_loss_rate: 0.02,
        };
        let score_batt_laptop = MasterFitnessScore::calculate(&battery_laptop, term);

        // 4. Smartphone on battery (lowest expected fitness)
        let smartphone = NodeFitnessProfile {
            did: "did:key:z6MkSmartphone0000000000000000004".to_string(),
            device_class: DeviceClass::Smartphone,
            is_charging: false,
            battery_percent: Some(40),
            available_ram: ByteSize::from_gib(4),
            packet_loss_rate: 0.05,
        };
        let score_phone = MasterFitnessScore::calculate(&smartphone, term);

        println!("Server Fitness: {:?}", score_server);
        println!("Plugged Laptop Fitness: {:?}", score_laptop);
        println!("Battery Laptop Fitness: {:?}", score_batt_laptop);
        println!("Smartphone Fitness: {:?}", score_phone);

        // Ranking assertion
        assert!(
            score_server > score_laptop,
            "Server must outrank plugged laptop"
        );
        assert!(
            score_laptop > score_batt_laptop,
            "Plugged laptop must outrank battery laptop"
        );
        assert!(
            score_batt_laptop > score_phone,
            "Battery laptop must outrank smartphone"
        );

        // Deterministic tie-breaker check on identical hardware with different DIDs
        let peer_a = NodeFitnessProfile {
            did: "did:key:z6MkAlpha000000000000000000000001".to_string(),
            device_class: DeviceClass::Desktop,
            is_charging: true,
            battery_percent: None,
            available_ram: ByteSize::from_gib(16),
            packet_loss_rate: 0.0,
        };
        let peer_b = NodeFitnessProfile {
            did: "did:key:z6MkBeta0000000000000000000000002".to_string(),
            device_class: DeviceClass::Desktop,
            is_charging: true,
            battery_percent: None,
            available_ram: ByteSize::from_gib(16),
            packet_loss_rate: 0.0,
        };
        let score_a = MasterFitnessScore::calculate(&peer_a, term);
        let score_b = MasterFitnessScore::calculate(&peer_b, term);
        assert_eq!(score_a.power_score, score_b.power_score);
        assert_eq!(score_a.device_score, score_b.device_score);
        assert_eq!(score_a.memory_score, score_b.memory_score);
        assert_ne!(
            score_a.tie_breaker, score_b.tie_breaker,
            "BLAKE3 tie-breaker must differ"
        );
        assert_ne!(score_a, score_b);
    }

    #[test]
    fn integration_five_node_cluster_election_failover_and_leader_transfer() {
        let now_ms = 1_700_000_000_000u64;

        // 1. Test isolated single node enters AutonomousSolo in < 500 ms
        let solo_start = Instant::now();
        let solo_did = "did:key:z6MkSoloNode0000000000000000000001";
        let mut solo_coord = ElectionCoordinator::new(solo_did, NodeRoleState::AutonomousSolo);
        let solo_profile = NodeFitnessProfile {
            did: solo_did.to_string(),
            device_class: DeviceClass::Laptop,
            is_charging: true,
            battery_percent: Some(100),
            available_ram: ByteSize::from_gib(16),
            packet_loss_rate: 0.0,
        };
        let solo_req = solo_coord
            .start_election(&solo_profile, now_ms)
            .expect("solo election start");
        assert_eq!(solo_req.candidate_did, solo_did);
        assert_eq!(solo_coord.role_state(), NodeRoleState::AutonomousSolo);
        assert_eq!(solo_coord.leader_did(), Some(solo_did));
        let solo_elapsed_ms = solo_start.elapsed().as_millis();
        println!(
            "AutonomousSolo initialization latency: {} ms (target < 500 ms)",
            solo_elapsed_ms
        );
        assert!(
            solo_elapsed_ms < 500,
            "Solo mode latency {} ms exceeds 500 ms ceiling",
            solo_elapsed_ms
        );

        // 2. Set up 5-node cluster
        let dids = [
            "did:key:z6MkNodeA_Server000000000000000001",
            "did:key:z6MkNodeB_Desktop00000000000000002",
            "did:key:z6MkNodeC_LaptopPlugged000000000003",
            "did:key:z6MkNodeD_LaptopBattery000000000004",
            "did:key:z6MkNodeE_PhoneBattery0000000000005",
        ];

        let profiles = [
            NodeFitnessProfile {
                did: dids[0].to_string(),
                device_class: DeviceClass::Server,
                is_charging: true,
                battery_percent: None,
                available_ram: ByteSize::from_gib(32),
                packet_loss_rate: 0.0,
            },
            NodeFitnessProfile {
                did: dids[1].to_string(),
                device_class: DeviceClass::Desktop,
                is_charging: true,
                battery_percent: None,
                available_ram: ByteSize::from_gib(32),
                packet_loss_rate: 0.0,
            },
            NodeFitnessProfile {
                did: dids[2].to_string(),
                device_class: DeviceClass::Laptop,
                is_charging: true,
                battery_percent: Some(100),
                available_ram: ByteSize::from_gib(16),
                packet_loss_rate: 0.0,
            },
            NodeFitnessProfile {
                did: dids[3].to_string(),
                device_class: DeviceClass::Laptop,
                is_charging: false,
                battery_percent: Some(80),
                available_ram: ByteSize::from_gib(8),
                packet_loss_rate: 0.01,
            },
            NodeFitnessProfile {
                did: dids[4].to_string(),
                device_class: DeviceClass::Smartphone,
                is_charging: false,
                battery_percent: Some(40),
                available_ram: ByteSize::from_gib(4),
                packet_loss_rate: 0.03,
            },
        ];

        let mut coords: Vec<ElectionCoordinator> = dids
            .iter()
            .map(|did| ElectionCoordinator::new(*did, NodeRoleState::Worker))
            .collect();

        // Cross-register peers across all 5 nodes
        for (i, coord) in coords.iter_mut().enumerate() {
            for (j, did) in dids.iter().enumerate() {
                if i != j {
                    coord.add_peer(*did);
                }
            }
            assert_eq!(coord.cluster_size(), 5);
            assert_eq!(coord.quorum_size(), 3);
        }

        // 3. Node A (highest fitness server) campaigns in term 1
        let election_timer = Instant::now();
        let vote_req = coords[0]
            .start_election(&profiles[0], now_ms)
            .expect("node a starts election");
        assert_eq!(coords[0].role_state(), NodeRoleState::Candidate);

        // Nodes B, C, D, E process vote request and grant vote to A
        for i in 1..5 {
            let vote_resp = coords[i].handle_request_vote(&vote_req, &profiles[i], now_ms);
            assert!(
                vote_resp.vote_granted,
                "Node {} must grant vote to Node A",
                i
            );
            let _ = coords[0].handle_vote_response(&vote_resp, now_ms);
        }

        // Node A has received 5 votes (> 3 quorum) -> Ascends to Master
        assert_eq!(coords[0].role_state(), NodeRoleState::Master);
        assert_eq!(coords[0].leader_did(), Some(dids[0]));
        let election_elapsed = election_timer.elapsed().as_millis();
        println!("Initial cluster election latency: {} ms", election_elapsed);

        // Broadcast leader heartbeat to all workers
        let heartbeat = LeaderHeartbeat {
            term: coords[0].current_term(),
            leader_did: dids[0].to_string(),
            leader_fitness: vote_req.candidate_fitness,
            active_job_count: 5,
            timestamp_epoch_ms: now_ms,
        };
        for coord in coords.iter_mut().skip(1) {
            let _ = coord.handle_leader_heartbeat(&heartbeat, now_ms);
            assert_eq!(coord.leader_did(), Some(dids[0]));
        }

        // 4. Test graceful LeaderTransfer from Node A to Node B (second highest fitness)
        let transfer_timer = Instant::now();
        let state_digest = [0x5au8; 32];
        let transfer_msg = coords[0]
            .initiate_leader_transfer(dids[1], state_digest, now_ms + 10)
            .expect("node a initiates transfer");

        assert_eq!(coords[0].role_state(), NodeRoleState::Worker);
        coords[1]
            .accept_leader_transfer(&transfer_msg, now_ms + 15)
            .expect("node b accepts transfer");
        assert_eq!(coords[1].role_state(), NodeRoleState::Master);
        assert_eq!(coords[1].leader_did(), Some(dids[1]));

        let transfer_elapsed_ms = transfer_timer.elapsed().as_millis();
        println!(
            "LeaderTransfer latency: {} ms (target < 250 ms)",
            transfer_elapsed_ms
        );
        assert!(
            transfer_elapsed_ms < 250,
            "LeaderTransfer {} ms exceeds 250 ms ceiling",
            transfer_elapsed_ms
        );

        // 5. Test unexpected failover: Node B crashes / disconnects. Next election must converge in < 3.0 s
        let failover_start = Instant::now();
        // Remove Node B from remaining peers
        for i in [0, 2, 3, 4] {
            coords[i].remove_peer(dids[1]);
        }

        // Node C (highest fitness remaining) detects timeout and starts term 3 election
        let req_c = coords[2]
            .start_election(&profiles[2], now_ms + 2500)
            .expect("node c campaigns");
        assert_eq!(coords[2].role_state(), NodeRoleState::Candidate);

        // Nodes D and E grant vote to C (Nodes A and C participate, A also grants vote if term > current)
        for i in [3, 4, 0] {
            let resp = coords[i].handle_request_vote(&req_c, &profiles[i], now_ms + 2500);
            assert!(resp.vote_granted, "Peer {} must grant vote to Node C", i);
            let _ = coords[2].handle_vote_response(&resp, now_ms + 2500);
        }

        // Node C has received 4/4 votes -> Ascends to Master with ZERO split brain
        assert_eq!(coords[2].role_state(), NodeRoleState::Master);
        assert_eq!(coords[2].leader_did(), Some(dids[2]));

        let failover_elapsed_ms = failover_start.elapsed().as_millis();
        println!(
            "Election failover latency: {} ms (target < 3000 ms)",
            failover_elapsed_ms
        );
        assert!(
            failover_elapsed_ms < 3000,
            "Failover elapsed {} ms exceeds 3000 ms ceiling",
            failover_elapsed_ms
        );

        // Verify zero split brain: exactly one leader exists
        let leaders: Vec<String> = [0, 2, 3, 4]
            .iter()
            .filter(|&&i| coords[i].role_state() == NodeRoleState::Master)
            .map(|&i| coords[i].local_did().to_string())
            .collect();
        assert_eq!(
            leaders,
            vec![dids[2].to_string()],
            "Strictly 1 master must exist"
        );

        // Export trace log to artifacts/evidence/stage-02/election_failover_trace.json
        let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        let root_artifacts = manifest_dir
            .parent()
            .and_then(|p| p.parent())
            .map(|p| p.join("artifacts").join("evidence").join("stage-02"))
            .unwrap_or_else(|| std::path::PathBuf::from("artifacts/evidence/stage-02"));

        fs::create_dir_all(&root_artifacts).expect("create evidence dir");
        let trace_path = root_artifacts.join("election_failover_trace.json");

        let trace_json = serde_json::json!({
            "subphase": "S2.P2.SP3",
            "title": "5-Node Cluster Election Failover & Leader Transfer Trace",
            "initial_leader": dids[0],
            "transferred_leader": dids[1],
            "failover_leader": dids[2],
            "leader_transfer_latency_ms": transfer_elapsed_ms,
            "failover_latency_ms": failover_elapsed_ms,
            "split_brain_count": 0,
            "solo_mode_latency_ms": solo_elapsed_ms,
            "cluster_nodes_evaluated": dids
        });

        fs::write(
            &trace_path,
            serde_json::to_string_pretty(&trace_json).expect("serialize trace"),
        )
        .expect("write trace file");
        assert!(trace_path.exists());
    }

    #[test]
    fn simulation_simultaneous_election_timeouts_under_packet_loss() {
        let now_ms = 1_700_000_100_000u64;
        let node_count = 7;
        let dids: Vec<String> = (0..node_count)
            .map(|i| format!("did:key:z6MkSimNode{:024}", i))
            .collect();

        let profiles: Vec<NodeFitnessProfile> = (0..node_count)
            .map(|i| NodeFitnessProfile {
                did: dids[i].clone(),
                device_class: if i == 0 {
                    DeviceClass::Workstation
                } else if i < 3 {
                    DeviceClass::Laptop
                } else {
                    DeviceClass::Smartphone
                },
                is_charging: i < 3,
                battery_percent: if i < 3 { Some(100) } else { Some(50) },
                available_ram: ByteSize::from_gib((32 - i * 4).max(4) as u64),
                packet_loss_rate: 0.15, // 15% packet loss
            })
            .collect();

        let mut coords: Vec<ElectionCoordinator> = dids
            .iter()
            .map(|did| ElectionCoordinator::new(did.clone(), NodeRoleState::Worker))
            .collect();

        for (i, coord) in coords.iter_mut().enumerate() {
            for (j, did) in dids.iter().enumerate() {
                if i != j {
                    coord.add_peer(did);
                }
            }
            assert_eq!(coord.cluster_size(), 7);
            assert_eq!(coord.quorum_size(), 4);
        }

        // All 7 nodes campaign simultaneously for term 1
        let mut vote_requests = Vec::new();
        for i in 0..node_count {
            let req = coords[i]
                .start_election(&profiles[i], now_ms)
                .expect("start campaign");
            vote_requests.push(req);
        }

        // Message exchange with simulated 15% packet loss
        // Simple PRNG to drop 15% of packets deterministically based on sender and receiver indices
        for i in 0..node_count {
            for j in 0..node_count {
                if i != j {
                    let drop = ((i * 13 + j * 7) % 100) < 15;
                    if !drop {
                        let resp =
                            coords[j].handle_request_vote(&vote_requests[i], &profiles[j], now_ms);
                        let _ = coords[i].handle_vote_response(&resp, now_ms);
                    }
                }
            }
        }

        // Count how many nodes reached Master in term 1
        let term1_masters: Vec<usize> = (0..node_count)
            .filter(|&i| coords[i].role_state() == NodeRoleState::Master)
            .collect();

        // Invariant: At most 1 leader can ever be elected per term (no split-brain)
        assert!(
            term1_masters.len() <= 1,
            "Term 1 must have at most 1 leader, got: {:?}",
            term1_masters
        );

        // If term 1 produced a tie or no quorum due to packet drops, run term 2 with highest-fitness node
        if term1_masters.is_empty() {
            println!("Term 1 split votes due to packet loss; advancing to Term 2...");
            let req2 = coords[0]
                .start_election(&profiles[0], now_ms + 1000)
                .expect("term 2 start");
            for j in 1..node_count {
                let resp = coords[j].handle_request_vote(&req2, &profiles[j], now_ms + 1000);
                let _ = coords[0].handle_vote_response(&resp, now_ms + 1000);
            }
            assert_eq!(
                coords[0].role_state(),
                NodeRoleState::Master,
                "Node 0 must achieve quorum in term 2"
            );
        } else {
            println!("Term 1 elected single leader node: {}", term1_masters[0]);
        }
    }
}
