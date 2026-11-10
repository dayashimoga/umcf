//! W3C Decentralized Identifier (`did:key`) and `DidDocument` implementation.
//! Strictly conforms to W3C DID Core 1.0 and multicodec standards (`0xed01` for Ed25519, `0xec01` for X25519).

use crate::error::IdentityError;
use crate::keys::NodeKeypair;
use serde::{Deserialize, Serialize};

/// W3C Multicodec prefix for Ed25519 public key (`0xed, 0x01`).
pub const ED25519_MULTICODEC_PREFIX: [u8; 2] = [0xed, 0x01];

/// W3C Multicodec prefix for X25519 public key (`0xec, 0x01`).
pub const X25519_MULTICODEC_PREFIX: [u8; 2] = [0xec, 0x01];

/// DID key utility functions for encoding and decoding `did:key:z6Mk...` identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DidKey;

impl DidKey {
    /// Encodes a 32-byte Ed25519 public key into a W3C `did:key:z6Mk...` identifier.
    #[must_use]
    pub fn from_ed25519_bytes(public_key: &[u8; 32]) -> String {
        let mut bytes = Vec::with_capacity(34);
        bytes.extend_from_slice(&ED25519_MULTICODEC_PREFIX);
        bytes.extend_from_slice(public_key);
        let b58 = bs58::encode(bytes).into_string();
        format!("did:key:z{b58}")
    }

    /// Decodes a W3C `did:key:z6Mk...` identifier back into a 32-byte Ed25519 public key.
    pub fn to_ed25519_bytes(did: &str) -> Result<[u8; 32], IdentityError> {
        let suffix = did
            .strip_prefix("did:key:z")
            .ok_or_else(|| IdentityError::MalformedDid("missing did:key:z prefix".to_string()))?;

        let decoded = bs58::decode(suffix)
            .into_vec()
            .map_err(|e| IdentityError::MalformedDid(format!("base58 decode failed: {e}")))?;

        if decoded.len() != 34 {
            return Err(IdentityError::MalformedDid(format!(
                "invalid byte length for ed25519 did:key: expected 34, got {}",
                decoded.len()
            )));
        }

        if decoded[0..2] != ED25519_MULTICODEC_PREFIX {
            return Err(IdentityError::UnsupportedDidMethod(format!(
                "expected multicodec prefix 0xed01, got {:02x}{:02x}",
                decoded[0], decoded[1]
            )));
        }

        let mut pk = [0u8; 32];
        pk.copy_from_slice(&decoded[2..34]);
        Ok(pk)
    }

    /// Encodes a 32-byte X25519 public key into a W3C `did:key:z6LS...` identifier.
    #[must_use]
    pub fn from_x25519_bytes(public_key: &[u8; 32]) -> String {
        let mut bytes = Vec::with_capacity(34);
        bytes.extend_from_slice(&X25519_MULTICODEC_PREFIX);
        bytes.extend_from_slice(public_key);
        let b58 = bs58::encode(bytes).into_string();
        format!("did:key:z{b58}")
    }

    /// Decodes a W3C `did:key:z6LS...` identifier back into a 32-byte X25519 public key.
    pub fn to_x25519_bytes(did: &str) -> Result<[u8; 32], IdentityError> {
        let suffix = did
            .strip_prefix("did:key:z")
            .ok_or_else(|| IdentityError::MalformedDid("missing did:key:z prefix".to_string()))?;

        let decoded = bs58::decode(suffix)
            .into_vec()
            .map_err(|e| IdentityError::MalformedDid(format!("base58 decode failed: {e}")))?;

        if decoded.len() != 34 {
            return Err(IdentityError::MalformedDid(format!(
                "invalid byte length for x25519 did:key: expected 34, got {}",
                decoded.len()
            )));
        }

        if decoded[0..2] != X25519_MULTICODEC_PREFIX {
            return Err(IdentityError::UnsupportedDidMethod(format!(
                "expected multicodec prefix 0xec01, got {:02x}{:02x}",
                decoded[0], decoded[1]
            )));
        }

        let mut pk = [0u8; 32];
        pk.copy_from_slice(&decoded[2..34]);
        Ok(pk)
    }
}

/// Verification Method entry in a W3C DID Document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VerificationMethod {
    pub id: String,
    #[serde(rename = "type")]
    pub type_: String,
    pub controller: String,
    pub public_key_multibase: String,
}

/// Full W3C DID Document representing a cryptographic node identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DidDocument {
    #[serde(rename = "@context")]
    pub context: Vec<String>,
    pub id: String,
    pub verification_method: Vec<VerificationMethod>,
    pub authentication: Vec<String>,
    pub assertion_method: Vec<String>,
    pub key_agreement: Vec<VerificationMethod>,
}

impl DidDocument {
    /// Generates a compliant W3C `DidDocument` from a `NodeKeypair`.
    #[must_use]
    pub fn from_keypair(keypair: &NodeKeypair) -> Self {
        let did = keypair.did_key();

        // Multibase formatted public keys
        let ed_bytes = keypair.ed25519_public_bytes();
        let mut ed_multicodec = Vec::with_capacity(34);
        ed_multicodec.extend_from_slice(&ED25519_MULTICODEC_PREFIX);
        ed_multicodec.extend_from_slice(&ed_bytes);
        let ed_multibase = format!("z{}", bs58::encode(ed_multicodec).into_string());

        let x_bytes = keypair.x25519_public_bytes();
        let mut x_multicodec = Vec::with_capacity(34);
        x_multicodec.extend_from_slice(&X25519_MULTICODEC_PREFIX);
        x_multicodec.extend_from_slice(&x_bytes);
        let x_multibase = format!("z{}", bs58::encode(x_multicodec).into_string());

        let ed_vm_id = format!("{did}#{did}");
        let x_vm_id = format!("{did}#key-agreement-1");

        let ed_vm = VerificationMethod {
            id: ed_vm_id.clone(),
            type_: "Ed25519VerificationKey2020".to_string(),
            controller: did.clone(),
            public_key_multibase: ed_multibase,
        };

        let x_vm = VerificationMethod {
            id: x_vm_id,
            type_: "X25519KeyAgreementKey2020".to_string(),
            controller: did.clone(),
            public_key_multibase: x_multibase,
        };

        Self {
            context: vec![
                "https://www.w3.org/ns/did/v1".to_string(),
                "https://w3id.org/security/suites/ed25519-2020/v1".to_string(),
                "https://w3id.org/security/suites/x25519-2020/v1".to_string(),
            ],
            id: did,
            verification_method: vec![ed_vm],
            authentication: vec![ed_vm_id.clone()],
            assertion_method: vec![ed_vm_id],
            key_agreement: vec![x_vm],
        }
    }

    /// Serializes the DID document to a pretty-printed JSON string.
    pub fn to_json_pretty(&self) -> Result<String, IdentityError> {
        serde_json::to_string_pretty(self)
            .map_err(|e| IdentityError::SerializationError(e.to_string()))
    }
}
