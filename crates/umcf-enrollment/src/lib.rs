//! `umcf-enrollment` — Short-lived QR enrollment tokens, matrix rendering, and secure candidate handshake.
//! Strictly enforces `#![forbid(unsafe_code)]` and zero-placeholder invariants.

#![forbid(unsafe_code)]

pub mod error;
pub mod handshake;
pub mod qr;
pub mod token;

pub use error::EnrollmentError;
pub use handshake::{CandidateEnroller, EnrollmentCoordinator, JoinRequest, ReplayProtectionCache};
pub use qr::QrRenderer;
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
}
