//! Classified Five-Tier Test Suite for GGUF Security, Provenance, Licensing & RBAC.
//! Covers [Unit], [Integration], [Simulation], [Hardware-Required], and [Manually-Verified].
//! Strictly enforces `#![forbid(unsafe_code)]` and zero placeholders.

#![forbid(unsafe_code)]

use std::fs::{self, OpenOptions};
use std::io::{Seek, SeekFrom, Write};
use std::time::Instant;

use ed25519_dalek::SigningKey;
use tempfile::TempDir;
use umcf_gguf::{
    create_test_gguf_bytes, ContentAddressedModelStore, Ed25519PublicKey, GgmlType,
    LicenseAcceptanceReceipt, LicenseCategory, LicensePolicyEngine, ModelAccessPolicy,
    ModelId, ModelIntegrityVerifier, ModelQuarantineManager, ModelVisibility,
    SignedModelAttestation, VerifyMode,
};

/// Helper to generate a deterministic Ed25519 keypair for test assertions.
fn create_test_signing_key(seed_byte: u8) -> SigningKey {
    let seed = [seed_byte; 32];
    SigningKey::from_bytes(&seed)
}

// =========================================================================
// 1. [Unit] Deterministic Unit Tests
// =========================================================================

#[test]
fn test_unit_ed25519_model_attestation_sign_and_verify() {
    let signing_key = create_test_signing_key(42);
    let model_id = ModelId([0x11; 32]);
    let header_hash = [0x22; 32];
    let tensor_hash = [0x33; 32];
    let license_id = "Llama-3-Community".to_string();
    let license_hash = [0x44; 32];

    let attestation = SignedModelAttestation::create_and_sign(
        model_id,
        header_hash,
        tensor_hash,
        license_id,
        license_hash,
        &signing_key,
    );

    // Verification must pass on untampered attestation
    attestation.verify().expect("attestation signature must be valid");

    // Tampering test: modify header_blake3
    let mut tampered = attestation.clone();
    tampered.header_blake3[0] ^= 0xFF;
    assert!(
        tampered.verify().is_err(),
        "Tampered attestation payload must fail signature verification"
    );

    // Key mismatch test: check against different public key
    let other_key = create_test_signing_key(99);
    let mut wrong_key_attestation = attestation;
    wrong_key_attestation.signer_public_key =
        Ed25519PublicKey(other_key.verifying_key().to_bytes());
    assert!(wrong_key_attestation.verify().is_err());
}

#[test]
fn test_unit_overlapping_or_out_of_bounds_tensor_offsets_rejected() {
    let temp = TempDir::new().expect("temp dir creation");
    let store = ContentAddressedModelStore::new(temp.path()).expect("store");

    // Create valid model with 4 layers
    let data = create_test_gguf_bytes("llama", 4, 2048, 256, 8, 4, GgmlType::Q4_K);
    let original_path = temp.path().join("source.gguf");
    fs::write(&original_path, &data).expect("write model");

    let manifest = store.ingest_local_file(&original_path).expect("ingest");

    // FastPreFlight on unmodified model must succeed
    let guard = ModelIntegrityVerifier::verify(&store, &manifest.model_id, VerifyMode::FastPreFlight, None)
        .expect("unmodified model passes preflight");
    assert_eq!(guard.model_id(), &manifest.model_id);

    // Corrupt tensor offset table: mutate tensor offset so it points beyond file bounds
    let model_file = store.model_file_path(&manifest.model_id);
    let mut corrupted_bytes = fs::read(&model_file).expect("read model");

    // Find first tensor entry and set offset to huge value
    for i in 0..corrupted_bytes.len() - 20 {
        if &corrupted_bytes[i..i + 17] == b"token_embd.weight" {
            // Offset is stored after name + n_dims + dims + ggml_type
            let offset_pos = i + 17 + 4 + 16 + 4;
            if offset_pos + 8 <= corrupted_bytes.len() {
                corrupted_bytes[offset_pos..offset_pos + 8].copy_from_slice(&(999_999_999u64).to_le_bytes());
                break;
            }
        }
    }
    fs::write(&model_file, &corrupted_bytes).expect("overwrite with corrupted offset");

    let err = ModelIntegrityVerifier::verify(&store, &manifest.model_id, VerifyMode::FastPreFlight, None)
        .expect_err("out of bounds offset must be rejected");
    assert!(
        err.to_string().contains("out of bounds") || err.to_string().contains("Range"),
        "Error must mention out of bounds range"
    );
}

#[test]
fn test_unit_license_policy_blocks_unaccepted_restricted_model() {
    let temp = TempDir::new().expect("temp dir creation");
    let policy_engine = LicensePolicyEngine::new(temp.path()).expect("license engine");

    let model_id = ModelId([0x55; 32]);

    // Permissive licenses require no acceptance receipt
    let apache_cat = LicensePolicyEngine::classify_license("Apache-2.0");
    assert_eq!(apache_cat, LicenseCategory::OpenPermissive);
    assert!(!LicensePolicyEngine::requires_explicit_acceptance(apache_cat));
    policy_engine
        .check_compliance(&model_id, apache_cat)
        .expect("permissive license compliant without receipt");

    // Community restricted licenses (e.g. Llama-3 Community)
    let llama_cat = LicensePolicyEngine::classify_license("Llama-3-Community");
    assert_eq!(llama_cat, LicenseCategory::CommunityRestricted);
    assert!(LicensePolicyEngine::requires_explicit_acceptance(llama_cat));

    // Initially unaccepted -> Must block compliance check
    let block_err = policy_engine
        .check_compliance(&model_id, llama_cat)
        .expect_err("unaccepted community license must be blocked");
    assert!(block_err.to_string().contains("License compliance violation"));

    // Record valid acceptance receipt
    let signing_key = create_test_signing_key(7);
    let receipt = LicenseAcceptanceReceipt::create_and_sign(
        model_id,
        "Llama-3-Community".to_string(),
        [0xAA; 32],
        "node-local-alpha".to_string(),
        1700000000000,
        &signing_key,
    );

    policy_engine
        .record_acceptance(receipt.clone())
        .expect("record receipt");
    assert!(policy_engine.is_accepted(&model_id));

    // After receipt recorded -> Compliance check must pass
    policy_engine
        .check_compliance(&model_id, llama_cat)
        .expect("license compliant after receipt recorded");

    // Verify stored receipt can be retrieved and validated
    let fetched = policy_engine
        .get_acceptance_receipt(&model_id)
        .expect("get receipt")
        .expect("receipt exists");
    assert_eq!(fetched.model_id, model_id);
    let pub_key = Ed25519PublicKey(signing_key.verifying_key().to_bytes());
    fetched.verify(&pub_key).expect("verify receipt signature");
}

#[test]
fn test_unit_private_local_only_policy_denies_remote_peer_access() {
    let model_id = ModelId([0x77; 32]);
    let private_policy = ModelAccessPolicy::private_local_only(model_id);

    // Local calls are always allowed
    private_policy
        .check_access("local-node", "Hybrid", true)
        .expect("local access always permitted");

    // Remote peer calls must be rejected with ModelNotFound (404) to prevent metadata leakage
    let remote_err = private_policy
        .check_access("remote-worker-1", "Worker", false)
        .expect_err("remote access must be denied");

    // Error must be ModelNotFound, not PermissionDenied
    match remote_err {
        umcf_gguf::StoreError::ModelNotFound(id) => assert_eq!(id, model_id),
        other => panic!("Expected ModelNotFound to prevent leakage, got: {:?}", other),
    }

    // Public shared policy permits remote peers
    let public_policy = ModelAccessPolicy::public_shared(model_id);
    public_policy
        .check_access("remote-worker-1", "Worker", false)
        .expect("public shared permits remote peer");
}

// =========================================================================
// 2. [Integration] End-to-End Quarantine and RBAC Tests
// =========================================================================

#[test]
fn test_integration_on_disk_corruption_triggers_automatic_quarantine() {
    let temp = TempDir::new().expect("temp dir");
    let store = ContentAddressedModelStore::new(temp.path()).expect("store");

    let data = create_test_gguf_bytes("llama", 4, 2048, 256, 8, 4, GgmlType::Q4_K);
    let path = temp.path().join("model.gguf");
    fs::write(&path, &data).expect("write file");

    let manifest = store.ingest_local_file(&path).expect("ingest");
    let model_id = manifest.model_id;

    // Model exists and is not quarantined
    assert!(store.has_model(&model_id));
    assert!(!ModelQuarantineManager::is_quarantined(&store, &model_id));

    // Simulate on-disk silent bit flip (e.g. disk sector corruption)
    let model_file = store.model_file_path(&model_id);
    let mut file = OpenOptions::new()
        .write(true)
        .open(&model_file)
        .expect("open file for bit flip");
    file.seek(SeekFrom::End(-10)).expect("seek near end");
    file.write_all(&[0xDE, 0xAD]).expect("corrupt bytes");
    drop(file);

    // FullCryptographicScrub detects chunk mismatch and automatically quarantines file!
    let scrub_err = ModelIntegrityVerifier::verify(&store, &model_id, VerifyMode::FullCryptographicScrub, None);
    assert!(scrub_err.is_err(), "Corrupted model must fail scrub");

    // Verify model is now in quarantine
    assert!(
        ModelQuarantineManager::is_quarantined(&store, &model_id),
        "Corrupted model must be moved to quarantine"
    );
    // Model is no longer present in active store
    assert!(!store.model_file_path(&model_id).exists());
}

#[test]
fn test_integration_rbac_capability_token_and_peer_role_validation() {
    let model_id = ModelId([0x88; 32]);
    let rbac_policy = ModelAccessPolicy {
        model_id,
        visibility: ModelVisibility::TrustedClusterPeers {
            allowed_roles: vec!["Master".to_string(), "Hybrid".to_string()],
            allowed_node_ids: vec!["trusted-phone-node".to_string()],
        },
        allowed_capabilities: vec!["inference:submit".to_string()],
    };

    // 1. Authorized role "Master"
    rbac_policy
        .check_access("any-master-node", "Master", false)
        .expect("Master role authorized");

    // 2. Specific authorized node ID
    rbac_policy
        .check_access("trusted-phone-node", "Worker", false)
        .expect("Specifically allowed node ID authorized");

    // 3. Unauthorized node and role
    let unauthorized_err = rbac_policy
        .check_access("random-external-node", "Worker", false)
        .expect_err("Unauthorized worker rejected");
    assert!(unauthorized_err.to_string().contains("not authorized"));
}

// =========================================================================
// 3. [Simulation] Sybil Peer Attestation Poisoning Defense
// =========================================================================

#[test]
fn test_simulation_sybil_peer_forged_attestation_defense() {
    let legitimate_signer = create_test_signing_key(10);
    let sybil_attacker = create_test_signing_key(99);

    let model_id = ModelId([0x99; 32]);
    let header_hash = [0xAA; 32];
    let tensor_hash = [0xBB; 32];
    let license_id = "Open-Weights".to_string();
    let license_hash = [0xCC; 32];

    // Sybil peer creates a forged attestation claiming it is from legitimate_signer
    let mut forged_attestation = SignedModelAttestation::create_and_sign(
        model_id,
        header_hash,
        tensor_hash,
        license_id,
        license_hash,
        &sybil_attacker, // Signed with attacker key
    );
    // Spoof legitimate public key in the attestation record
    forged_attestation.signer_public_key =
        Ed25519PublicKey(legitimate_signer.verifying_key().to_bytes());

    // Defense verification: signature verification against spoofed key must fail!
    let verification_result = forged_attestation.verify();
    assert!(
        verification_result.is_err(),
        "Forged attestation from Sybil attacker must be rejected"
    );
}

// =========================================================================
// 4. [Hardware-Required] FastPreFlight Verification Latency Benchmark (< 5 ms)
// =========================================================================

#[test]
fn test_hardware_fast_preflight_verification_latency_benchmark() {
    let temp = TempDir::new().expect("temp dir");
    let store = ContentAddressedModelStore::new(temp.path()).expect("store");

    // 8 layers, 2048 ctx, 512 embd (~8 MiB file)
    let data = create_test_gguf_bytes("llama", 8, 2048, 512, 8, 4, GgmlType::Q4_K);
    let path = temp.path().join("benchmark_model.gguf");
    fs::write(&path, &data).expect("write file");

    let manifest = store.ingest_local_file(&path).expect("ingest");

    // Warm-up
    let _ = ModelIntegrityVerifier::verify(&store, &manifest.model_id, VerifyMode::FastPreFlight, None);

    // Measure pre-flight validation latency
    let start = Instant::now();
    let guard = ModelIntegrityVerifier::verify(&store, &manifest.model_id, VerifyMode::FastPreFlight, None)
        .expect("preflight verification");
    let elapsed = start.elapsed();
    let elapsed_ms = elapsed.as_secs_f64() * 1000.0;

    println!(
        "[HARDWARE-REQUIRED] FastPreFlight verification completed in {:.3} ms (< 5.0 ms target). Guard model: {}",
        elapsed_ms, guard.model_id()
    );

    assert!(
        elapsed_ms < 5.0,
        "FastPreFlight latency ({:.3} ms) must be strictly under 5.0 ms",
        elapsed_ms
    );
}

// =========================================================================
// 5. [Manually-Verified] UI License Acceptance & Security Audit Log
// =========================================================================

#[test]
fn test_manual_ui_license_acceptance_and_security_audit_log() {
    let temp = TempDir::new().expect("temp dir");
    let engine = LicensePolicyEngine::new(temp.path()).expect("license engine");

    let model_id = ModelId([0x44; 32]);
    let signing_key = create_test_signing_key(1);

    // Simulated user acceptance in UI dialog
    let receipt = LicenseAcceptanceReceipt::create_and_sign(
        model_id,
        "Gemma-Terms-of-Use".to_string(),
        [0x77; 32],
        "local-admin-device".to_string(),
        1700000050000,
        &signing_key,
    );

    engine.record_acceptance(receipt.clone()).expect("record receipt");
    assert!(engine.is_accepted(&model_id));

    // Audit verification
    let audit_receipt = engine
        .get_acceptance_receipt(&model_id)
        .expect("query receipt")
        .expect("receipt found");
    assert_eq!(audit_receipt.license_id, "Gemma-Terms-of-Use");
    assert_eq!(audit_receipt.accepted_by_node, "local-admin-device");
}
