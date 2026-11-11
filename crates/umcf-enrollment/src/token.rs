//! Compact, cryptographically signed, short-lived QR enrollment tokens.
//! Strictly enforces `#![forbid(unsafe_code)]` and zero-placeholder invariants.

use crate::error::EnrollmentError;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use ed25519_dalek::{Signature, VerifyingKey};
use rand_core::{OsRng, RngCore};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::time::{SystemTime, UNIX_EPOCH};
use umcf_core::NodeRole;
use umcf_identity::{DidKey, NodeKeypair, DOMAIN_SEPARATION_PREFIX};

/// Default token time-to-live: 180 seconds (3 minutes).
pub const DEFAULT_TOKEN_TTL_MS: u64 = 180_000;

/// Maximum allowable token time-to-live: 600 seconds (10 minutes).
pub const MAX_TOKEN_TTL_MS: u64 = 600_000;

/// URI scheme prefix for UMCF enrollment tokens.
pub const URI_SCHEME_PREFIX: &str = "umcf://enroll/v1/";

/// Domain separation prefix for enrollment token signing.
pub const ENROLLMENT_TOKEN_DOMAIN: &[u8] = b"UMCF-ENROLL-v1:";

/// Serde-compatible binary wire format for compact URL representation (< 320 ASCII characters).
#[derive(Serialize, Deserialize)]
struct CompactTokenWire {
    cluster_id: String,
    issuer_pk: [u8; 32],
    ephemeral_x25519_pk: [u8; 32],
    bootstrap_endpoints: Vec<SocketAddr>,
    one_time_nonce: [u8; 16],
    expires_at_ms: u64,
    proposed_role: u8,
    #[serde(with = "raw_signature_serde")]
    issuer_ed25519_sig: [u8; 64],
}

/// Cryptographically signed, short-lived enrollment token presented via QR code.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnrollmentToken {
    /// Mesh cluster identifier this token authorizes joining.
    pub cluster_id: String,
    /// W3C did:key of the issuing coordinator node.
    pub issuer_did: String,
    /// Raw 32-byte Ed25519 public key of the issuing coordinator.
    pub issuer_public_key: [u8; 32],
    /// Ephemeral X25519 public key generated for this enrollment session.
    pub ephemeral_x25519_pk: [u8; 32],
    /// Reachable network addresses for the coordinator.
    pub bootstrap_endpoints: Vec<SocketAddr>,
    /// Unique 128-bit single-use nonce for replay defense.
    pub one_time_nonce: [u8; 16],
    /// Expiration epoch timestamp in milliseconds.
    pub expires_at_ms: u64,
    /// Operating role proposed for the joining candidate node.
    pub proposed_role: NodeRole,
    /// Ed25519 signature by the coordinator over the canonical token digest.
    #[serde(with = "signature_serde")]
    pub issuer_ed25519_sig: [u8; 64],
}

impl EnrollmentToken {
    /// Generates a new signed `EnrollmentToken` with cryptographically secure random nonce.
    pub fn generate(
        issuer: &NodeKeypair,
        cluster_id: impl Into<String>,
        ephemeral_x25519_pk: [u8; 32],
        bootstrap_endpoints: Vec<SocketAddr>,
        proposed_role: NodeRole,
        ttl_ms: Option<u64>,
    ) -> Result<Self, EnrollmentError> {
        let ttl = ttl_ms.unwrap_or(DEFAULT_TOKEN_TTL_MS);
        if ttl > MAX_TOKEN_TTL_MS {
            return Err(EnrollmentError::MaxTtlExceeded {
                requested_ms: ttl,
                max_ms: MAX_TOKEN_TTL_MS,
            });
        }

        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        let expires_at_ms = now_ms.saturating_add(ttl);

        let mut one_time_nonce = [0u8; 16];
        OsRng.fill_bytes(&mut one_time_nonce);

        let cluster_id = cluster_id.into();
        let issuer_pk = issuer.ed25519_public_bytes();
        let issuer_did = issuer.did_key();

        let digest = Self::compute_digest(
            &cluster_id,
            &issuer_pk,
            &ephemeral_x25519_pk,
            &bootstrap_endpoints,
            &one_time_nonce,
            expires_at_ms,
            proposed_role,
        );

        let issuer_ed25519_sig = issuer.sign(&digest);

        Ok(Self {
            cluster_id,
            issuer_did,
            issuer_public_key: issuer_pk,
            ephemeral_x25519_pk,
            bootstrap_endpoints,
            one_time_nonce,
            expires_at_ms,
            proposed_role,
            issuer_ed25519_sig,
        })
    }

    /// Verifies the token's cryptographic signature against its embedded issuer public key.
    pub fn verify_signature(&self) -> Result<(), EnrollmentError> {
        let digest = Self::compute_digest(
            &self.cluster_id,
            &self.issuer_public_key,
            &self.ephemeral_x25519_pk,
            &self.bootstrap_endpoints,
            &self.one_time_nonce,
            self.expires_at_ms,
            self.proposed_role,
        );

        let verifying_key = VerifyingKey::from_bytes(&self.issuer_public_key)
            .map_err(|_| EnrollmentError::InvalidSignature)?;

        let mut signed_payload = Vec::with_capacity(DOMAIN_SEPARATION_PREFIX.len() + digest.len());
        signed_payload.extend_from_slice(DOMAIN_SEPARATION_PREFIX);
        signed_payload.extend_from_slice(&digest);

        let sig = Signature::from_bytes(&self.issuer_ed25519_sig);
        verifying_key
            .verify_strict(&signed_payload, &sig)
            .map_err(|_| EnrollmentError::InvalidSignature)
    }

    /// Checks if the token has expired relative to `current_time_ms`.
    #[must_use]
    pub const fn is_expired(&self, current_time_ms: u64) -> bool {
        current_time_ms > self.expires_at_ms
    }

    /// Serializes the token into a compact URI string (`umcf://enroll/v1/<base64url>`).
    /// Guarantees that total URI length is `<= 320` ASCII characters.
    pub fn to_uri(&self) -> Result<String, EnrollmentError> {
        let role_byte = match self.proposed_role {
            NodeRole::Master => 0,
            NodeRole::Worker => 1,
            NodeRole::Hybrid => 2,
        };

        let wire = CompactTokenWire {
            cluster_id: self.cluster_id.clone(),
            issuer_pk: self.issuer_public_key,
            ephemeral_x25519_pk: self.ephemeral_x25519_pk,
            bootstrap_endpoints: self.bootstrap_endpoints.clone(),
            one_time_nonce: self.one_time_nonce,
            expires_at_ms: self.expires_at_ms,
            proposed_role: role_byte,
            issuer_ed25519_sig: self.issuer_ed25519_sig,
        };

        let bytes = bincode::serialize(&wire)
            .map_err(|e| EnrollmentError::SerializationError(e.to_string()))?;
        let encoded = URL_SAFE_NO_PAD.encode(bytes);

        let uri = format!("{}{}", URI_SCHEME_PREFIX, encoded);
        Ok(uri)
    }

    /// Deserializes and validates an `EnrollmentToken` from a URI string.
    pub fn from_uri(uri: &str) -> Result<Self, EnrollmentError> {
        if !uri.starts_with(URI_SCHEME_PREFIX) {
            return Err(EnrollmentError::MalformedUri(
                "URI does not start with expected umcf://enroll/v1/ prefix".to_string(),
            ));
        }

        let encoded = &uri[URI_SCHEME_PREFIX.len()..];
        let bytes = URL_SAFE_NO_PAD
            .decode(encoded)
            .map_err(|e| EnrollmentError::MalformedUri(e.to_string()))?;

        let wire: CompactTokenWire = bincode::deserialize(&bytes)
            .map_err(|e| EnrollmentError::MalformedUri(e.to_string()))?;

        let proposed_role = match wire.proposed_role {
            0 => NodeRole::Master,
            1 => NodeRole::Worker,
            2 => NodeRole::Hybrid,
            other => {
                return Err(EnrollmentError::MalformedUri(format!(
                    "invalid role tag {other}"
                )))
            }
        };

        let issuer_did = DidKey::from_ed25519_bytes(&wire.issuer_pk);

        let token = Self {
            cluster_id: wire.cluster_id,
            issuer_did,
            issuer_public_key: wire.issuer_pk,
            ephemeral_x25519_pk: wire.ephemeral_x25519_pk,
            bootstrap_endpoints: wire.bootstrap_endpoints,
            one_time_nonce: wire.one_time_nonce,
            expires_at_ms: wire.expires_at_ms,
            proposed_role,
            issuer_ed25519_sig: wire.issuer_ed25519_sig,
        };

        token.verify_signature()?;
        Ok(token)
    }

    /// Computes canonical 256-bit BLAKE3 digest of signable token contents.
    #[must_use]
    fn compute_digest(
        cluster_id: &str,
        issuer_pk: &[u8; 32],
        ephemeral_x25519_pk: &[u8; 32],
        bootstrap_endpoints: &[SocketAddr],
        one_time_nonce: &[u8; 16],
        expires_at_ms: u64,
        role: NodeRole,
    ) -> [u8; 32] {
        let mut hasher = blake3::Hasher::new();
        hasher.update(ENROLLMENT_TOKEN_DOMAIN);
        hasher.update(cluster_id.as_bytes());
        hasher.update(issuer_pk);
        hasher.update(ephemeral_x25519_pk);

        for ep in bootstrap_endpoints {
            hasher.update(ep.to_string().as_bytes());
        }

        hasher.update(one_time_nonce);
        hasher.update(&expires_at_ms.to_be_bytes());
        let role_byte = match role {
            NodeRole::Master => 0u8,
            NodeRole::Worker => 1u8,
            NodeRole::Hybrid => 2u8,
        };
        hasher.update(&[role_byte]);

        *hasher.finalize().as_bytes()
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

mod raw_signature_serde {
    use serde::{Deserializer, Serializer};

    pub fn serialize<S>(sig: &[u8; 64], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_bytes(sig)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<[u8; 64], D::Error>
    where
        D: Deserializer<'de>,
    {
        struct Visitor;
        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = [u8; 64];

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("a 64-byte slice")
            }

            fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                if v.len() == 64 {
                    let mut arr = [0u8; 64];
                    arr.copy_from_slice(v);
                    Ok(arr)
                } else {
                    Err(serde::de::Error::invalid_length(v.len(), &self))
                }
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                let mut arr = [0u8; 64];
                for (i, elem) in arr.iter_mut().enumerate() {
                    *elem = seq
                        .next_element()?
                        .ok_or_else(|| serde::de::Error::invalid_length(i, &self))?;
                }
                Ok(arr)
            }
        }
        deserializer.deserialize_bytes(Visitor)
    }
}
