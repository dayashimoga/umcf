//! Mutual attestation protocol binding RFC 8446 TLS 1.3 keying exporters to node identity.
//! Strictly enforces `#![forbid(unsafe_code)]` and zero-placeholder invariants.

use crate::did::DidKey;
use crate::error::IdentityError;
use crate::keys::{NodeKeypair, DOMAIN_SEPARATION_PREFIX};
use ed25519_dalek::{Signature, VerifyingKey};
use rand_core::{OsRng, RngCore};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

/// RFC 8446 TLS 1.3 exporter label bound to mutual attestation proofs.
pub const TLS_EXPORTER_LABEL: &[u8] = b"EXPORTER-UMCF-ATTEST-v1";

/// Domain separation prefix for mutual attestation signatures.
pub const ATTESTATION_DOMAIN: &[u8] = b"UMCF-ATTEST-v1:";

/// Challenge sent by a verifier node to initiate 2-RTT mutual attestation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttestationChallenge {
    /// W3C did:key of the challenger node.
    pub challenger_did: String,
    /// Cryptographically secure 256-bit random nonce.
    pub challenge_nonce: [u8; 32],
    /// Unix epoch timestamp in milliseconds when challenge was issued.
    pub issued_at_epoch_ms: u64,
}

/// Cryptographic proof produced by a node binding TLS exporter material and hardware identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttestationProof {
    /// W3C did:key of the proving node.
    pub prover_did: String,
    /// Nonce echoed back from the `AttestationChallenge`.
    pub challenge_nonce: [u8; 32],
    /// 32-byte keying material exported from RFC 8446 TLS 1.3 channel.
    pub tls_exporter_binding: [u8; 32],
    /// Cryptographic BLAKE3 digest representing node hardware capabilities.
    pub hardware_fingerprint: [u8; 32],
    /// Unix epoch timestamp in milliseconds when proof was generated.
    pub timestamp_epoch_ms: u64,
    /// Ed25519 signature over canonical attestation digest.
    #[serde(with = "signature_serde")]
    pub signature: [u8; 64],
}

mod signature_serde {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(sig: &[u8; 64], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&hex_encode(sig))
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<[u8; 64], D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let bytes = hex_decode(&s).map_err(serde::de::Error::custom)?;
        if bytes.len() != 64 {
            return Err(serde::de::Error::custom("expected 64 bytes for signature"));
        }
        let mut out = [0u8; 64];
        out.copy_from_slice(&bytes);
        Ok(out)
    }

    fn hex_encode(data: &[u8]) -> String {
        data.iter().map(|b| format!("{b:02x}")).collect()
    }

    fn hex_decode(s: &str) -> Result<Vec<u8>, String> {
        if !s.len().is_multiple_of(2) {
            return Err("odd hex string length".into());
        }
        (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16).map_err(|e| e.to_string()))
            .collect()
    }
}

/// 2-RTT Mutual Attestation Protocol preventing MITM relay, replay, and impersonation.
#[derive(Debug, Clone, Copy, Default)]
pub struct MutualAttestationProtocol;

impl MutualAttestationProtocol {
    /// Creates a new fresh challenge containing a 256-bit CSPRNG nonce.
    #[must_use]
    pub fn create_challenge(challenger: &NodeKeypair) -> AttestationChallenge {
        let mut nonce = [0u8; 32];
        OsRng.fill_bytes(&mut nonce);
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        AttestationChallenge {
            challenger_did: challenger.did_key(),
            challenge_nonce: nonce,
            issued_at_epoch_ms: now_ms,
        }
    }

    /// Generates a signed `AttestationProof` binding the TLS exporter and hardware fingerprint.
    #[must_use]
    pub fn create_proof(
        prover: &NodeKeypair,
        challenge: &AttestationChallenge,
        tls_exporter_binding: [u8; 32],
        hardware_fingerprint: [u8; 32],
    ) -> AttestationProof {
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        let prover_did = prover.did_key();
        let digest = Self::compute_attestation_digest(
            &prover_did,
            &challenge.challenge_nonce,
            &tls_exporter_binding,
            &hardware_fingerprint,
            now_ms,
        );

        let signature = prover.sign(&digest);

        AttestationProof {
            prover_did,
            challenge_nonce: challenge.challenge_nonce,
            tls_exporter_binding,
            hardware_fingerprint,
            timestamp_epoch_ms: now_ms,
            signature,
        }
    }

    /// Verifies an `AttestationProof` against the original challenge and expected TLS exporter.
    pub fn verify_proof(
        proof: &AttestationProof,
        original_challenge: &AttestationChallenge,
        expected_tls_exporter: &[u8; 32],
        prover_public_key: &[u8; 32],
        max_clock_skew_ms: u64,
        current_epoch_ms: u64,
    ) -> Result<(), IdentityError> {
        // 1. Verify nonce matches original challenge
        if proof.challenge_nonce != original_challenge.challenge_nonce {
            return Err(IdentityError::AttestationFailed(
                "challenge nonce mismatch; possible replay or reflection attempt".to_string(),
            ));
        }

        // 2. Verify TLS 1.3 channel binding
        if &proof.tls_exporter_binding != expected_tls_exporter {
            return Err(IdentityError::AttestationFailed(
                "TLS 1.3 exporter keying material mismatch; MITM relay detected".to_string(),
            ));
        }

        // 3. Verify timestamp freshness within allowed clock skew
        let min_valid = original_challenge
            .issued_at_epoch_ms
            .saturating_sub(max_clock_skew_ms);
        let max_valid = current_epoch_ms.saturating_add(max_clock_skew_ms);
        if proof.timestamp_epoch_ms < min_valid || proof.timestamp_epoch_ms > max_valid {
            return Err(IdentityError::AttestationFailed(format!(
                "proof timestamp out of valid window: proof={}, window=[{}, {}]",
                proof.timestamp_epoch_ms, min_valid, max_valid
            )));
        }

        // 4. Verify prover DID corresponds to provided Ed25519 public key
        let expected_did = DidKey::from_ed25519_bytes(prover_public_key);
        if proof.prover_did != expected_did {
            return Err(IdentityError::SenderMismatch {
                expected: expected_did,
                actual: proof.prover_did.clone(),
            });
        }

        // 5. Verify Ed25519 signature over canonical digest
        let digest = Self::compute_attestation_digest(
            &proof.prover_did,
            &proof.challenge_nonce,
            &proof.tls_exporter_binding,
            &proof.hardware_fingerprint,
            proof.timestamp_epoch_ms,
        );

        let verifying_key = VerifyingKey::from_bytes(prover_public_key)
            .map_err(|e| IdentityError::InvalidPublicKey(e.to_string()))?;

        // Format signed payload with domain separation prefix
        let mut signed_payload = Vec::with_capacity(DOMAIN_SEPARATION_PREFIX.len() + digest.len());
        signed_payload.extend_from_slice(DOMAIN_SEPARATION_PREFIX);
        signed_payload.extend_from_slice(&digest);

        let sig = Signature::from_bytes(&proof.signature);
        verifying_key
            .verify_strict(&signed_payload, &sig)
            .map_err(|_| IdentityError::InvalidSignature)
    }

    /// Computes canonical 256-bit BLAKE3 digest over all attestation fields.
    #[must_use]
    pub fn compute_attestation_digest(
        prover_did: &str,
        challenge_nonce: &[u8; 32],
        tls_exporter: &[u8; 32],
        hardware_fingerprint: &[u8; 32],
        timestamp_epoch_ms: u64,
    ) -> [u8; 32] {
        let mut hasher = blake3::Hasher::new();
        hasher.update(ATTESTATION_DOMAIN);
        hasher.update(prover_did.as_bytes());
        hasher.update(challenge_nonce);
        hasher.update(tls_exporter);
        hasher.update(hardware_fingerprint);
        hasher.update(&timestamp_epoch_ms.to_be_bytes());
        *hasher.finalize().as_bytes()
    }
}
