//! Secure candidate enrollment handshake, replay protection, and certificate issuance.
//! Strictly enforces `#![forbid(unsafe_code)]` and zero-placeholder invariants.

use crate::error::EnrollmentError;
use crate::token::EnrollmentToken;
use chacha20poly1305::aead::{Aead, KeyInit};
use chacha20poly1305::{XChaCha20Poly1305, XNonce};
use ed25519_dalek::{Signature, VerifyingKey};
use rand_core::{OsRng, RngCore};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use umcf_core::{HardwareCapabilityDescriptor, NodeRole};
use umcf_identity::{
    ClusterCa, DidDocument, NodeCertificate, NodeKeypair, DOMAIN_SEPARATION_PREFIX,
};
use x25519_dalek::{PublicKey as X25519Public, StaticSecret as X25519Secret};

/// Single-use nonce cache defending against token replay attacks.
#[derive(Debug, Default)]
pub struct ReplayProtectionCache {
    seen_nonces: HashSet<[u8; 16]>,
}

impl ReplayProtectionCache {
    /// Creates an empty replay protection cache.
    #[must_use]
    pub fn new() -> Self {
        Self {
            seen_nonces: HashSet::new(),
        }
    }

    /// Verifies that the nonce has not been seen previously, and marks it as used.
    pub fn check_and_mark(&mut self, nonce: &[u8; 16]) -> Result<(), EnrollmentError> {
        if self.seen_nonces.contains(nonce) {
            return Err(EnrollmentError::TokenReplayed(*nonce));
        }
        self.seen_nonces.insert(*nonce);
        Ok(())
    }
}

/// Request submitted by a candidate node seeking to join the cluster.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JoinRequest {
    /// W3C did:key of the joining candidate node.
    pub candidate_did: String,
    /// Raw 32-byte Ed25519 public key of candidate.
    pub candidate_public_key: [u8; 32],
    /// Candidate's ephemeral X25519 public key for Diffie-Hellman session encryption.
    pub candidate_ephemeral_x25519_pk: [u8; 32],
    /// W3C DID Document of the candidate.
    pub did_document: DidDocument,
    /// Probed hardware capability profile of the candidate node.
    pub hardware_capabilities: HardwareCapabilityDescriptor,
    /// The token nonce being redeemed.
    pub token_nonce: [u8; 16],
    /// AEAD encrypted payload containing candidate authentication challenge.
    pub encrypted_payload: Vec<u8>,
    /// 24-byte random XChaCha20-Poly1305 nonce used for payload encryption.
    pub encryption_nonce: [u8; 24],
    /// Ed25519 signature over canonical join request digest.
    #[serde(with = "signature_serde")]
    pub candidate_signature: [u8; 64],
}

/// Map storing active ephemeral secrets indexed by single-use token nonce.
type ActiveTokenMap = HashMap<[u8; 16], (X25519Secret, [u8; 32])>;

/// Coordinator service managing cluster enrollment, token issuance, and join request processing.
pub struct EnrollmentCoordinator {
    cluster_id: String,
    coordinator_keypair: Arc<NodeKeypair>,
    ca: Arc<ClusterCa>,
    replay_cache: Arc<Mutex<ReplayProtectionCache>>,
    active_tokens: Arc<Mutex<ActiveTokenMap>>,
}

impl std::fmt::Debug for EnrollmentCoordinator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EnrollmentCoordinator")
            .field("cluster_id", &self.cluster_id)
            .field("coordinator_did", &self.coordinator_keypair.did_key())
            .finish_non_exhaustive()
    }
}

impl EnrollmentCoordinator {
    /// Creates a new `EnrollmentCoordinator` bound to a cluster identity, keypair, and CA.
    #[must_use]
    pub fn new(
        cluster_id: impl Into<String>,
        coordinator_keypair: Arc<NodeKeypair>,
        ca: Arc<ClusterCa>,
    ) -> Self {
        Self {
            cluster_id: cluster_id.into(),
            coordinator_keypair,
            ca,
            replay_cache: Arc::new(Mutex::new(ReplayProtectionCache::new())),
            active_tokens: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Issues a new signed, short-lived `EnrollmentToken` and registers its ephemeral secret.
    pub fn issue_token(
        &self,
        bootstrap_endpoints: Vec<SocketAddr>,
        proposed_role: NodeRole,
        ttl_ms: Option<u64>,
    ) -> Result<EnrollmentToken, EnrollmentError> {
        let mut ephemeral_secret_bytes = [0u8; 32];
        OsRng.fill_bytes(&mut ephemeral_secret_bytes);
        let ephemeral_secret = X25519Secret::from(ephemeral_secret_bytes);
        let ephemeral_public = X25519Public::from(&ephemeral_secret);
        let ephemeral_public_bytes = *ephemeral_public.as_bytes();

        let token = EnrollmentToken::generate(
            &self.coordinator_keypair,
            &self.cluster_id,
            ephemeral_public_bytes,
            bootstrap_endpoints,
            proposed_role,
            ttl_ms,
        )?;

        let mut active = self
            .active_tokens
            .lock()
            .map_err(|e| EnrollmentError::HandshakeFailed(e.to_string()))?;
        active.insert(
            token.one_time_nonce,
            (ephemeral_secret, ephemeral_public_bytes),
        );

        Ok(token)
    }

    /// Evaluates a candidate's `JoinRequest`, enforces replay protection, decrypts payload,
    /// and issues a signed X.509 `NodeCertificate`.
    pub fn process_join_request(
        &self,
        token: &EnrollmentToken,
        request: &JoinRequest,
        current_time_ms: u64,
    ) -> Result<NodeCertificate, EnrollmentError> {
        // 1. Verify token signature
        token.verify_signature()?;

        // 2. Enforce token expiration
        if token.is_expired(current_time_ms) {
            return Err(EnrollmentError::TokenExpired {
                expires_at_ms: token.expires_at_ms,
                now_ms: current_time_ms,
            });
        }

        // 3. Enforce single-use replay protection
        {
            let mut cache = self
                .replay_cache
                .lock()
                .map_err(|e| EnrollmentError::HandshakeFailed(e.to_string()))?;
            cache.check_and_mark(&token.one_time_nonce)?;
        }

        // 4. Retrieve ephemeral session secret
        let (ephemeral_secret, _) = {
            let mut active = self
                .active_tokens
                .lock()
                .map_err(|e| EnrollmentError::HandshakeFailed(e.to_string()))?;
            active
                .remove(&token.one_time_nonce)
                .ok_or(EnrollmentError::TokenReplayed(token.one_time_nonce))?
        };

        // 5. Verify JoinRequest cryptographic signature
        let digest = Self::compute_join_digest(
            &request.candidate_did,
            &request.candidate_public_key,
            &request.candidate_ephemeral_x25519_pk,
            &request.token_nonce,
            &request.encryption_nonce,
            &request.encrypted_payload,
        );

        let verifying_key = VerifyingKey::from_bytes(&request.candidate_public_key)
            .map_err(|_| EnrollmentError::InvalidSignature)?;

        let mut signed_payload = Vec::with_capacity(DOMAIN_SEPARATION_PREFIX.len() + digest.len());
        signed_payload.extend_from_slice(DOMAIN_SEPARATION_PREFIX);
        signed_payload.extend_from_slice(&digest);

        let sig = Signature::from_bytes(&request.candidate_signature);
        verifying_key
            .verify_strict(&signed_payload, &sig)
            .map_err(|_| EnrollmentError::InvalidSignature)?;

        // 6. Complete X25519 Diffie-Hellman key agreement & decrypt payload
        let candidate_x25519_pk = X25519Public::from(request.candidate_ephemeral_x25519_pk);
        let shared_secret = ephemeral_secret.diffie_hellman(&candidate_x25519_pk);

        let cipher_key = blake3::derive_key("UMCF-ENROLL-AEAD-KEY-v1", shared_secret.as_bytes());
        let cipher = XChaCha20Poly1305::new_from_slice(&cipher_key)
            .map_err(|e| EnrollmentError::HandshakeFailed(e.to_string()))?;

        let nonce = XNonce::from_slice(&request.encryption_nonce);
        let decrypted_bytes = cipher
            .decrypt(nonce, request.encrypted_payload.as_slice())
            .map_err(|e| {
                EnrollmentError::HandshakeFailed(format!("AEAD decryption failed: {e}"))
            })?;

        if decrypted_bytes != request.token_nonce {
            return Err(EnrollmentError::HandshakeFailed(
                "decrypted challenge nonce does not match token nonce".to_string(),
            ));
        }

        // 7. Issue signed leaf NodeCertificate directly for candidate public key and DID
        let cert = self
            .ca
            .issue_certificate_for_public_key(
                &request.candidate_public_key,
                &request.candidate_did,
                token.proposed_role,
                30,
            )
            .map_err(|e| EnrollmentError::IdentityError(e.to_string()))?;

        Ok(cert)
    }

    /// Computes canonical 256-bit BLAKE3 digest of signable join request contents.
    #[must_use]
    pub fn compute_join_digest(
        candidate_did: &str,
        candidate_public_key: &[u8; 32],
        candidate_ephemeral_pk: &[u8; 32],
        token_nonce: &[u8; 16],
        encryption_nonce: &[u8; 24],
        encrypted_payload: &[u8],
    ) -> [u8; 32] {
        let mut hasher = blake3::Hasher::new();
        hasher.update(b"UMCF-JOIN-v1:");
        hasher.update(candidate_did.as_bytes());
        hasher.update(candidate_public_key);
        hasher.update(candidate_ephemeral_pk);
        hasher.update(token_nonce);
        hasher.update(encryption_nonce);
        hasher.update(encrypted_payload);
        *hasher.finalize().as_bytes()
    }
}

/// Helper representing a candidate node scanning a token and generating a `JoinRequest`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CandidateEnroller;

impl CandidateEnroller {
    /// Creates a verified `JoinRequest` binding the scanned `EnrollmentToken` and local capabilities.
    pub fn prepare_join_request(
        candidate: &NodeKeypair,
        token: &EnrollmentToken,
        hardware_capabilities: HardwareCapabilityDescriptor,
        current_time_ms: u64,
    ) -> Result<JoinRequest, EnrollmentError> {
        // 1. Verify token signature and validity
        token.verify_signature()?;
        if token.is_expired(current_time_ms) {
            return Err(EnrollmentError::TokenExpired {
                expires_at_ms: token.expires_at_ms,
                now_ms: current_time_ms,
            });
        }

        // 2. Generate ephemeral X25519 keypair for session encryption
        let mut ephemeral_secret_bytes = [0u8; 32];
        OsRng.fill_bytes(&mut ephemeral_secret_bytes);
        let candidate_x25519_secret = X25519Secret::from(ephemeral_secret_bytes);
        let candidate_ephemeral_pk = X25519Public::from(&candidate_x25519_secret);

        // 3. Diffie-Hellman key agreement with token's ephemeral public key
        let token_x25519_pk = X25519Public::from(token.ephemeral_x25519_pk);
        let shared_secret = candidate_x25519_secret.diffie_hellman(&token_x25519_pk);

        let cipher_key = blake3::derive_key("UMCF-ENROLL-AEAD-KEY-v1", shared_secret.as_bytes());
        let cipher = XChaCha20Poly1305::new_from_slice(&cipher_key)
            .map_err(|e| EnrollmentError::HandshakeFailed(e.to_string()))?;

        let mut encryption_nonce = [0u8; 24];
        OsRng.fill_bytes(&mut encryption_nonce);
        let nonce = XNonce::from_slice(&encryption_nonce);

        // Encrypt token nonce as proof of shared secret knowledge
        let encrypted_payload = cipher
            .encrypt(nonce, token.one_time_nonce.as_slice())
            .map_err(|e| EnrollmentError::HandshakeFailed(e.to_string()))?;

        let candidate_did = candidate.did_key();
        let candidate_pk = candidate.ed25519_public_bytes();
        let candidate_ephemeral_pk_bytes = *candidate_ephemeral_pk.as_bytes();

        let digest = EnrollmentCoordinator::compute_join_digest(
            &candidate_did,
            &candidate_pk,
            &candidate_ephemeral_pk_bytes,
            &token.one_time_nonce,
            &encryption_nonce,
            &encrypted_payload,
        );

        let candidate_signature = candidate.sign(&digest);
        let did_document = DidDocument::from_keypair(candidate);

        Ok(JoinRequest {
            candidate_did,
            candidate_public_key: candidate_pk,
            candidate_ephemeral_x25519_pk: candidate_ephemeral_pk_bytes,
            did_document,
            hardware_capabilities,
            token_nonce: token.one_time_nonce,
            encrypted_payload,
            encryption_nonce,
            candidate_signature,
        })
    }
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
