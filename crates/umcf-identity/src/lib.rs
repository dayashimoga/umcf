//! `umcf-identity` — Cryptographic device identity, Ed25519/X25519 keypairs, and W3C DID document generation.
//! Strictly enforces `#![forbid(unsafe_code)]` and zero-placeholder invariants.

#![forbid(unsafe_code)]

pub mod did;
pub mod envelope_auth;
pub mod error;
pub mod keys;

pub use did::{DidDocument, DidKey, VerificationMethod};
pub use envelope_auth::{sign_envelope, verify_envelope};
pub use error::IdentityError;
pub use keys::NodeKeypair;

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;
    use std::time::Instant;
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
        let keypair = NodeKeypair::generate();
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
