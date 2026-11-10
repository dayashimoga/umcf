//! `umcf-identity` — Cryptographic device identity, Ed25519/X25519 keypairs,
//! encrypted local vault, and W3C DID document generation.
//! Strictly enforces `#![forbid(unsafe_code)]` and zero-placeholder invariants.

#![forbid(unsafe_code)]

pub mod did;
pub mod envelope_auth;
pub mod error;
pub mod keys;
pub mod os_keystore;
pub mod storage;
pub mod vault;

pub use did::{DidDocument, DidKey, VerificationMethod};
pub use envelope_auth::{sign_envelope, verify_envelope};
pub use error::IdentityError;
pub use keys::NodeKeypair;
pub use os_keystore::{KeystoreBackendKind, PlatformKeyStore};
pub use storage::{HistoricalKeyRecord, KeyStorageBackend, PersistedIdentity};
pub use vault::{LocalEncryptedVault, VaultArgon2Params, VAULT_HEADER_SIZE, VAULT_MAGIC};

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;
    use std::time::Instant;
    use tempfile::tempdir;
    use umcf_proto::envelope::MessageFlags;
    use umcf_proto::{MessageKind, PlaneKind, WireEnvelope};

    #[test]
    fn unit_ed25519_generation_and_x25519_diffie_hellman() {
        // 1. Generate keypairs for Alice and Bob
        let alice = NodeKeypair::generate();
        let bob = NodeKeypair::generate();

        assert_ne!(alice.ed25519_public_bytes(), bob.ed25519_public_bytes());
        assert_ne!(alice.x25519_public_bytes(), bob.x25519_public_bytes());

        // 2. Perform mutual Diffie-Hellman key agreement
        let alice_shared = alice.diffie_hellman(bob.x25519_public());
        let bob_shared = bob.diffie_hellman(alice.x25519_public());

        assert_eq!(alice_shared, bob_shared);
        assert_ne!(alice_shared, [0u8; 32]);

        // 3. Test domain-separated signing & verification
        let message = b"Hello distributed compute fabric!";
        let sig = alice.sign(message);
        assert!(alice.verify(message, &sig).is_ok());
        assert!(bob.verify(message, &sig).is_err());
    }

    #[test]
    fn unit_did_key_encoding_and_decoding() {
        let keypair = NodeKeypair::generate();
        let did = keypair.did_key();

        // Must start with did:key:z6Mk
        assert!(did.starts_with("did:key:z6Mk"));

        // Roundtrip decoding
        let decoded_pk = DidKey::to_ed25519_bytes(&did).expect("decode ed25519 did:key");
        assert_eq!(decoded_pk, keypair.ed25519_public_bytes());

        // X25519 did:key
        let x_did = DidKey::from_x25519_bytes(&keypair.x25519_public_bytes());
        assert!(x_did.starts_with("did:key:z6LS"));
        let decoded_x_pk = DidKey::to_x25519_bytes(&x_did).expect("decode x25519 did:key");
        assert_eq!(decoded_x_pk, keypair.x25519_public_bytes());
    }

    #[tokio::test]
    async fn unit_vault_create_unlock_wrong_passphrase_rejection_and_rotation() {
        let dir = tempdir().expect("create tempdir");
        let vault_path = dir.path().join(".umcf_vault.enc");
        let correct_passphrase = "Correct-Battery-Horse-Staple-42!";
        let wrong_passphrase = "Wrong-Passphrase-Attempt-99!";

        let vault = LocalEncryptedVault::new(
            vault_path.clone(),
            correct_passphrase,
            VaultArgon2Params::fast_for_testing(),
        );

        // 1. Initial state: vault doesn't exist
        assert!(!vault.exists());
        assert!(vault.load_identity().await.expect("load on empty").is_none());

        // 2. Store new identity
        let initial_keypair = NodeKeypair::generate();
        let initial_did = initial_keypair.did_key();
        let initial_identity = PersistedIdentity::new(initial_keypair, 1700000000000);
        vault
            .store_identity(&initial_identity)
            .await
            .expect("store identity");
        assert!(vault.exists());

        // 3. Unlock with wrong passphrase must fail with VaultIntegrityFailure
        let wrong_vault = LocalEncryptedVault::new(
            vault_path.clone(),
            wrong_passphrase,
            VaultArgon2Params::fast_for_testing(),
        );
        let wrong_res = wrong_vault.load_identity().await;
        assert!(wrong_res.is_err());
        match wrong_res {
            Err(IdentityError::VaultIntegrityFailure(_)) => {}
            other => panic!("Expected VaultIntegrityFailure, got {:?}", other),
        }

        // 4. Unlock with correct passphrase succeeds and verifies identity
        let loaded = vault
            .load_identity()
            .await
            .expect("load with correct passphrase")
            .expect("identity exists");
        assert_eq!(loaded.did(), initial_did);
        assert_eq!(loaded.generation(), 1);
        assert!(loaded.history().is_empty());

        // 5. Test key rotation preserving history
        let new_keypair = NodeKeypair::generate();
        let new_did = new_keypair.did_key();
        assert_ne!(initial_did, new_did);

        let rotated = vault
            .rotate_identity(new_keypair)
            .await
            .expect("rotate identity");
        assert_eq!(rotated.did(), new_did);
        assert_eq!(rotated.generation(), 2);
        assert_eq!(rotated.history().len(), 1);
        assert_eq!(rotated.history()[0].did, initial_did);

        // 6. Test historical signature verification
        let msg = b"Historical receipt from generation 1";
        let historical_sig = initial_identity.keypair().sign(msg);
        let is_valid = rotated
            .verify_signature_with_history(
                msg,
                &historical_sig,
                &initial_identity.keypair().ed25519_public_bytes(),
            )
            .expect("verify historical signature");
        assert!(is_valid);

        // Active signature verification
        let msg2 = b"Active receipt from generation 2";
        let active_sig = rotated.keypair().sign(msg2);
        let is_active_valid = rotated
            .verify_signature_with_history(
                msg2,
                &active_sig,
                &rotated.keypair().ed25519_public_bytes(),
            )
            .expect("verify active signature");
        assert!(is_active_valid);

        // 7. Revocation
        vault.revoke_identity().await.expect("revoke identity");
        assert!(!vault.exists());
    }

    #[tokio::test]
    async fn integration_vault_identity_continuity_across_simulated_restart() {
        let dir = tempdir().expect("create tempdir");
        let vault_path = dir.path().join("node_vault.enc");
        let passphrase = "Master-Vault-Cluster-Passphrase#2026";

        let original_did;
        let original_public_key;

        // Process 1: Initialize node, save identity, shutdown
        {
            let keystore = PlatformKeyStore::auto_detect(
                vault_path.clone(),
                passphrase,
                VaultArgon2Params::fast_for_testing(),
            );

            let keypair = NodeKeypair::generate();
            original_did = keypair.did_key();
            original_public_key = keypair.ed25519_public_bytes();

            let identity = PersistedIdentity::new(keypair, 1700000000000);
            keystore
                .store_identity(&identity)
                .await
                .expect("process 1 store identity");
        }

        // Process 2: Restart process, reload identity from disk, verify continuity
        {
            let keystore = PlatformKeyStore::auto_detect(
                vault_path.clone(),
                passphrase,
                VaultArgon2Params::fast_for_testing(),
            );

            let loaded = keystore
                .load_identity()
                .await
                .expect("process 2 load identity")
                .expect("identity must be present");

            assert_eq!(loaded.did(), original_did);
            assert_eq!(loaded.keypair().ed25519_public_bytes(), original_public_key);

            // Verify signing capability continuity
            let message = b"Transaction executed after node reboot";
            let sig = loaded.keypair().sign(message);
            assert!(loaded.keypair().verify(message, &sig).is_ok());
        }
    }

    #[tokio::test]
    async fn simulation_vault_fuzzing_500_iterations_tamper_detection() {
        let dir = tempdir().expect("create tempdir");
        let vault_path = dir.path().join("fuzz_vault.enc");
        let passphrase = "fuzzing-test-passphrase-aead";

        let vault = LocalEncryptedVault::new(
            vault_path.clone(),
            passphrase,
            VaultArgon2Params::fast_for_testing(),
        );

        let keypair = NodeKeypair::generate();
        let identity = PersistedIdentity::new(keypair, 1700000000000);
        vault
            .store_identity(&identity)
            .await
            .expect("store test identity");

        let original_bytes = fs::read(&vault_path).expect("read vault bytes");
        assert!(original_bytes.len() > VAULT_HEADER_SIZE);

        let total_bytes = original_bytes.len();
        let iterations = 500;

        for i in 0..iterations {
            let mut tampered_bytes = original_bytes.clone();
            // Corrupt byte at deterministic offsets across header, nonce, and ciphertext
            let offset = (i * 17 + 3) % total_bytes;
            tampered_bytes[offset] ^= 0x55;

            fs::write(&vault_path, &tampered_bytes).expect("write tampered vault");

            let res = vault.load_identity().await;
            assert!(
                res.is_err(),
                "Tampering at offset {} failed to trigger integrity error!",
                offset
            );

            match res {
                Err(IdentityError::VaultIntegrityFailure(_)) => {}
                Err(other) => panic!(
                    "Expected VaultIntegrityFailure on iteration {}, got: {:?}",
                    i, other
                ),
                Ok(_) => panic!("Tampered vault decrypted successfully!"),
            }
        }
    }

    #[test]
    fn integration_sign_and_verify_wire_envelope() {
        let keypair = NodeKeypair::generate();

        let mut env = WireEnvelope::builder(
            PlaneKind::ControlPlane,
            MessageKind::Heartbeat,
            keypair.did_key(),
        )
        .sequence_id(1)
        .payload(b"test-auth-payload".to_vec())
        .build()
        .expect("build envelope");

        // Sign envelope
        sign_envelope(&keypair, &mut env).expect("sign envelope");
        assert_ne!(env.signature, [0u8; 64]);

        // Verify envelope
        let res = verify_envelope(&env);
        assert!(res.is_ok());
    }

    #[test]
    fn integration_export_sample_did_document() {
        let seed = [42u8; 32];
        let keypair = NodeKeypair::from_seed(&seed);
        let doc = DidDocument::from_keypair(&keypair);

        let json = doc.to_json_pretty().expect("pretty print did doc");
        assert!(json.contains("Ed25519VerificationKey2020"));
        assert!(json.contains("X25519KeyAgreementKey2020"));

        let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        let root_artifacts = manifest_dir
            .parent()
            .and_then(|p| p.parent())
            .map(|p| p.join("artifacts").join("evidence").join("stage-02"))
            .unwrap_or_else(|| std::path::PathBuf::from("artifacts/evidence/stage-02"));

        fs::create_dir_all(&root_artifacts).expect("create evidence dir");
        let sample_path = root_artifacts.join("did_document_sample.json");
        fs::write(&sample_path, json).expect("write sample did doc");
        assert!(sample_path.exists());
    }

    #[test]
    fn simulation_tamper_detection_and_signature_rejection() {
        let alice = NodeKeypair::generate();
        let eve = NodeKeypair::generate();

        let mut env = WireEnvelope::builder(
            PlaneKind::ControlPlane,
            MessageKind::ScheduleTask,
            alice.did_key(),
        )
        .sequence_id(1)
        .payload(b"legitimate-workload-payload".to_vec())
        .build()
        .expect("build envelope");

        sign_envelope(&alice, &mut env).expect("sign legitimate envelope");

        // 1. Mutate payload -> should fail verification
        let mut tampered_payload_env = env.clone();
        tampered_payload_env.payload = b"malicious-injected-payload".to_vec();
        assert_eq!(
            verify_envelope(&tampered_payload_env),
            Err(IdentityError::InvalidSignature)
        );

        // 2. Mutate sender DID (impersonation attempt) -> should fail
        let mut impersonated_env = env.clone();
        impersonated_env.sender_node_id = eve.did_key();
        assert_eq!(
            verify_envelope(&impersonated_env),
            Err(IdentityError::InvalidSignature)
        );

        // 3. Mutate flags (tampering with control headers) -> should fail
        let mut tampered_flags_env = env.clone();
        tampered_flags_env.flags = MessageFlags::COMPRESSED_ZSTD;
        assert_eq!(
            verify_envelope(&tampered_flags_env),
            Err(IdentityError::InvalidSignature)
        );
    }

    #[test]
    fn unit_envelope_signing_and_verification_latency_microbenchmark() {
        let keypair = NodeKeypair::generate();
        let mut env = WireEnvelope::builder(
            PlaneKind::ControlPlane,
            MessageKind::Heartbeat,
            keypair.did_key(),
        )
        .sequence_id(1)
        .payload(vec![42u8; 1024])
        .build()
        .expect("build envelope");

        // Warmup
        for _ in 0..10 {
            sign_envelope(&keypair, &mut env).expect("sign");
            verify_envelope(&env).expect("verify");
        }

        // Measure 100 iterations
        let start = Instant::now();
        let iterations = 100;
        for _ in 0..iterations {
            sign_envelope(&keypair, &mut env).expect("sign");
            verify_envelope(&env).expect("verify");
        }
        let elapsed = start.elapsed();
        let per_op_micros = elapsed.as_micros() as f64 / (iterations as f64 * 2.0);

        println!(
            "Sign/Verify microbenchmark: {:.2} µs per op (target < 80 µs)",
            per_op_micros
        );
        // Acceptance criterion: < 80 µs per operation in release mode (relaxed in unoptimized debug mode)
        let ceiling = if cfg!(debug_assertions) {
            10_000.0
        } else {
            80.0
        };
        assert!(
            per_op_micros < ceiling,
            "Latency exceeds benchmark ceiling: {:.2} µs (ceiling: {:.2} µs)",
            per_op_micros,
            ceiling
        );
    }
}
