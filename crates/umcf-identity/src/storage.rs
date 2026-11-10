//! Storage interfaces and persisted identity models for UMCF nodes.
//! Strictly adheres to `#![forbid(unsafe_code)]` and zeroize memory protection.

use crate::error::IdentityError;
use crate::keys::NodeKeypair;
use ed25519_dalek::VerifyingKey;
use serde::{Deserialize, Serialize};
use zeroize::{Zeroize, ZeroizeOnDrop};

/// Record of a previous public key used by the node prior to key rotation.
/// Retained to verify historical receipts, signed audit logs, and past task completions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HistoricalKeyRecord {
    /// 32-byte Ed25519 verifying key.
    pub ed25519_public: [u8; 32],
    /// 32-byte X25519 public key.
    pub x25519_public: [u8; 32],
    /// W3C did:key string for this historical key.
    pub did: String,
    /// Unix epoch timestamp in milliseconds when this key was retired.
    pub rotated_at_epoch_ms: u64,
    /// Generation counter at retirement.
    pub generation: u64,
}

/// Serde-compatible payload for encrypted vault persistence.
#[derive(Serialize, Deserialize)]
struct PersistedIdentityWire {
    seed: [u8; 32],
    did: String,
    created_at_epoch_ms: u64,
    generation: u64,
    history: Vec<HistoricalKeyRecord>,
}

/// In-memory representation of a node's complete cryptographic identity.
/// Contains the current active keypair, creation metadata, rotation generation,
/// and full history of prior public keys.
pub struct PersistedIdentity {
    keypair: NodeKeypair,
    did: String,
    created_at_epoch_ms: u64,
    generation: u64,
    history: Vec<HistoricalKeyRecord>,
}

impl Drop for PersistedIdentity {
    fn drop(&mut self) {
        // NodeKeypair drops and zeroizes its own seed
    }
}

impl ZeroizeOnDrop for PersistedIdentity {}

impl std::fmt::Debug for PersistedIdentity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PersistedIdentity")
            .field("did", &self.did)
            .field("created_at_epoch_ms", &self.created_at_epoch_ms)
            .field("generation", &self.generation)
            .field("history_count", &self.history.len())
            .finish_non_exhaustive()
    }
}

impl PersistedIdentity {
    /// Creates a new `PersistedIdentity` from an active keypair and creation timestamp.
    #[must_use]
    pub fn new(keypair: NodeKeypair, created_at_epoch_ms: u64) -> Self {
        let did = keypair.did_key();
        Self {
            keypair,
            did,
            created_at_epoch_ms,
            generation: 1,
            history: Vec::new(),
        }
    }

    /// Returns a reference to the active `NodeKeypair`.
    #[must_use]
    pub const fn keypair(&self) -> &NodeKeypair {
        &self.keypair
    }

    /// Returns the active W3C `did:key` string.
    #[must_use]
    pub fn did(&self) -> &str {
        &self.did
    }

    /// Returns the creation epoch timestamp in milliseconds.
    #[must_use]
    pub const fn created_at_epoch_ms(&self) -> u64 {
        self.created_at_epoch_ms
    }

    /// Returns the current rotation generation.
    #[must_use]
    pub const fn generation(&self) -> u64 {
        self.generation
    }

    /// Returns the history of previously used public keys.
    #[must_use]
    pub fn history(&self) -> &[HistoricalKeyRecord] {
        &self.history
    }

    /// Rotates the active keypair to `new_keypair`, archiving the retired key into history.
    pub fn rotate(&mut self, new_keypair: NodeKeypair, rotated_at_epoch_ms: u64) {
        let retired_record = HistoricalKeyRecord {
            ed25519_public: self.keypair.ed25519_public_bytes(),
            x25519_public: self.keypair.x25519_public_bytes(),
            did: self.did.clone(),
            rotated_at_epoch_ms,
            generation: self.generation,
        };

        self.history.push(retired_record);
        self.generation = self.generation.saturating_add(1);
        self.did = new_keypair.did_key();
        self.keypair = new_keypair;
    }

    /// Verifies a signature against either the current active key or any historical public key.
    /// Returns `Ok(true)` if verified, `Ok(false)` if public key is unknown,
    /// or `Err(IdentityError::InvalidSignature)` if signature is invalid.
    pub fn verify_signature_with_history(
        &self,
        message: &[u8],
        signature: &[u8; 64],
        public_key: &[u8; 32],
    ) -> Result<bool, IdentityError> {
        // 1. Check active key
        if &self.keypair.ed25519_public_bytes() == public_key {
            self.keypair.verify(message, signature)?;
            return Ok(true);
        }

        // 2. Check historical keys
        for rec in &self.history {
            if &rec.ed25519_public == public_key {
                let verifying_key = VerifyingKey::from_bytes(public_key)
                    .map_err(|e| IdentityError::InvalidPublicKey(e.to_string()))?;
                NodeKeypair::verify_with_key(&verifying_key, message, signature)?;
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Serializes the identity into a canonical JSON payload for encrypted vault persistence.
    pub fn to_encrypted_payload(&self) -> Result<Vec<u8>, IdentityError> {
        let wire = PersistedIdentityWire {
            seed: *self.keypair.seed(),
            did: self.did.clone(),
            created_at_epoch_ms: self.created_at_epoch_ms,
            generation: self.generation,
            history: self.history.clone(),
        };
        serde_json::to_vec(&wire).map_err(|e| IdentityError::SerializationError(e.to_string()))
    }

    /// Deserializes a `PersistedIdentity` from a decrypted JSON payload.
    pub fn from_decrypted_payload(payload: &[u8]) -> Result<Self, IdentityError> {
        let mut wire: PersistedIdentityWire = serde_json::from_slice(payload)
            .map_err(|e| IdentityError::SerializationError(e.to_string()))?;

        let keypair = NodeKeypair::from_seed(&wire.seed);
        wire.seed.zeroize();

        Ok(Self {
            keypair,
            did: wire.did,
            created_at_epoch_ms: wire.created_at_epoch_ms,
            generation: wire.generation,
            history: wire.history,
        })
    }
}

/// Asynchronous backend abstraction for storing, rotating, and loading device cryptographic identity.
#[async_trait::async_trait]
pub trait KeyStorageBackend: Send + Sync {
    /// Stores the provided identity into persistent storage.
    async fn store_identity(&self, identity: &PersistedIdentity) -> Result<(), IdentityError>;

    /// Loads the stored identity if one exists.
    async fn load_identity(&self) -> Result<Option<PersistedIdentity>, IdentityError>;

    /// Rotates the active keypair to `new_keypair`, updating storage and preserving history.
    async fn rotate_identity(
        &self,
        new_keypair: NodeKeypair,
    ) -> Result<PersistedIdentity, IdentityError>;

    /// Completely revokes and destroys stored cryptographic keys and history.
    async fn revoke_identity(&self) -> Result<(), IdentityError>;
}
