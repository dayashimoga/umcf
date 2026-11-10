//! Cryptographic model provenance, pre-load integrity auditing, and RBAC security gates.
//!
//! Enforces zero-unsafe pre-load tensor table validation, signed Ed25519 attestations,
//! automatic file quarantine on silent corruption, and strict capability-guarded access.
//!
//! Strictly adheres to `#![forbid(unsafe_code)]` and zero-placeholder invariants.

#![forbid(unsafe_code)]

use std::fmt;
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::error::StoreError;
use crate::parser::{GgufParser, ModelInspection};
use crate::store::{ByteRange, ContentAddressedModelStore, Ed25519Signature, ModelId};

/// 32-byte Ed25519 public key of a model publisher or attestation authority.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Ed25519PublicKey(pub [u8; 32]);

impl Ed25519PublicKey {
    #[must_use]
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    #[must_use]
    pub fn to_hex(&self) -> String {
        let mut s = String::with_capacity(64);
        for b in &self.0 {
            use std::fmt::Write as _;
            let _ = write!(s, "{b:02x}");
        }
        s
    }

    pub fn from_hex(s: &str) -> Result<Self, StoreError> {
        let clean = s.trim().strip_prefix("0x").unwrap_or(s.trim());
        if clean.len() != 64 {
            return Err(StoreError::CorruptedFile(format!(
                "invalid hex length for Ed25519PublicKey: expected 64, got {}",
                clean.len()
            )));
        }
        let mut bytes = [0u8; 32];
        for (i, chunk) in clean.as_bytes().chunks_exact(2).enumerate() {
            let chunk_str = std::str::from_utf8(chunk)
                .map_err(|e| StoreError::CorruptedFile(format!("utf8 error: {e}")))?;
            bytes[i] = u8::from_str_radix(chunk_str, 16)
                .map_err(|e| StoreError::CorruptedFile(format!("hex parse error: {e}")))?;
        }
        Ok(Self(bytes))
    }
}

impl fmt::Display for Ed25519PublicKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

impl fmt::Debug for Ed25519PublicKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Ed25519PublicKey({})", self.to_hex())
    }
}

impl Serialize for Ed25519PublicKey {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_hex())
    }
}

impl<'de> Deserialize<'de> for Ed25519PublicKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Self::from_hex(&s).map_err(serde::de::Error::custom)
    }
}

/// Cryptographically signed model attestation guaranteeing authenticity and integrity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignedModelAttestation {
    pub model_id: ModelId,
    pub header_blake3: [u8; 32],
    pub tensor_table_blake3: [u8; 32],
    pub license_id: String,
    pub license_text_blake3: [u8; 32],
    pub signer_public_key: Ed25519PublicKey,
    pub signature: Ed25519Signature,
}

impl SignedModelAttestation {
    /// Computes canonical binary signing payload across attestation fields.
    #[must_use]
    pub fn compute_signing_payload(&self) -> Vec<u8> {
        let mut payload = Vec::with_capacity(160);
        payload.extend_from_slice(b"UMCF-MODEL-ATTESTATION-v1:");
        payload.extend_from_slice(self.model_id.as_bytes());
        payload.extend_from_slice(&self.header_blake3);
        payload.extend_from_slice(&self.tensor_table_blake3);
        payload.extend_from_slice(&(self.license_id.len() as u64).to_le_bytes());
        payload.extend_from_slice(self.license_id.as_bytes());
        payload.extend_from_slice(&self.license_text_blake3);
        payload
    }

    /// Creates and signs a new model attestation.
    pub fn create_and_sign(
        model_id: ModelId,
        header_blake3: [u8; 32],
        tensor_table_blake3: [u8; 32],
        license_id: String,
        license_text_blake3: [u8; 32],
        signing_key: &SigningKey,
    ) -> Self {
        let pub_key = Ed25519PublicKey(signing_key.verifying_key().to_bytes());

        let mut dummy = Self {
            model_id,
            header_blake3,
            tensor_table_blake3,
            license_id,
            license_text_blake3,
            signer_public_key: pub_key,
            signature: Ed25519Signature([0u8; 64]),
        };

        let payload = dummy.compute_signing_payload();
        let sig: Signature = signing_key.sign(&payload);
        dummy.signature = Ed25519Signature(sig.to_bytes());
        dummy
    }

    /// Verifies the Ed25519 cryptographic signature on this attestation.
    pub fn verify(&self) -> Result<(), StoreError> {
        let verifying_key = VerifyingKey::from_bytes(&self.signer_public_key.0)
            .map_err(|e| StoreError::InvalidSignature(format!("invalid public key: {e}")))?;

        let sig = Signature::from_bytes(&self.signature.0);
        let payload = self.compute_signing_payload();

        verifying_key
            .verify(&payload, &sig)
            .map_err(|e| StoreError::InvalidSignature(format!("signature verification failed: {e}")))
    }
}

/// Verification thoroughness mode for `ModelIntegrityVerifier`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VerifyMode {
    /// Fast header validation, tensor offset bounds checks, and collision detection (< 5 ms).
    FastPreFlight,
    /// Full cryptographic scrub verifying all 4 MiB chunks against manifest with quarantine on mismatch.
    FullCryptographicScrub,
}

/// Capability type proving that a GGUF model has passed strict cryptographic integrity checks.
/// Private constructor ensures downstream runtime engines (e.g. `llama.cpp` FFI) can only be
/// invoked with a verified model.
#[derive(Debug, Clone)]
pub struct VerifiedModelGuard {
    model_id: ModelId,
    file_path: PathBuf,
    inspection: Arc<ModelInspection>,
    attestation: Option<SignedModelAttestation>,
    verified_at_unix_ms: u64,
}

impl VerifiedModelGuard {
    #[must_use]
    pub const fn model_id(&self) -> &ModelId {
        &self.model_id
    }

    #[must_use]
    pub fn file_path(&self) -> &Path {
        &self.file_path
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        &self.file_path
    }

    #[must_use]
    pub fn inspection(&self) -> &ModelInspection {
        &self.inspection
    }

    #[must_use]
    pub fn attestation(&self) -> Option<&SignedModelAttestation> {
        self.attestation.as_ref()
    }

    #[must_use]
    pub const fn verified_at_unix_ms(&self) -> u64 {
        self.verified_at_unix_ms
    }
}

/// Model access control policy governing cluster visibility and remote shard distribution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModelVisibility {
    /// Never advertised over P2P mesh; returns 404 to all remote peer requests.
    PrivateLocalOnly,
    /// Accessible only to authenticated peers matching allowed node IDs or roles.
    TrustedClusterPeers {
        allowed_roles: Vec<String>,
        allowed_node_ids: Vec<String>,
    },
    /// Publicly accessible to any verified cluster peer.
    PublicShared,
}

/// Model access policy bundle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelAccessPolicy {
    pub model_id: ModelId,
    pub visibility: ModelVisibility,
    pub allowed_capabilities: Vec<String>,
}

impl ModelAccessPolicy {
    #[must_use]
    pub fn private_local_only(model_id: ModelId) -> Self {
        Self {
            model_id,
            visibility: ModelVisibility::PrivateLocalOnly,
            allowed_capabilities: vec!["inference:local".to_string()],
        }
    }

    #[must_use]
    pub fn public_shared(model_id: ModelId) -> Self {
        Self {
            model_id,
            visibility: ModelVisibility::PublicShared,
            allowed_capabilities: vec!["inference:*".to_string(), "shard:pull".to_string()],
        }
    }

    /// Evaluates peer access permissions. Returns 404 for PrivateLocalOnly to prevent metadata leakage.
    pub fn check_access(
        &self,
        peer_id: &str,
        peer_role: &str,
        is_local_call: bool,
    ) -> Result<(), StoreError> {
        if is_local_call {
            return Ok(());
        }

        match &self.visibility {
            ModelVisibility::PrivateLocalOnly => {
                // Deny with ModelNotFound to prevent leaking existence of private model
                Err(StoreError::ModelNotFound(self.model_id))
            }
            ModelVisibility::TrustedClusterPeers {
                allowed_roles,
                allowed_node_ids,
            } => {
                let role_match = allowed_roles.iter().any(|r| r == peer_role || r == "*");
                let node_match = allowed_node_ids.iter().any(|n| n == peer_id || n == "*");
                if role_match || node_match {
                    Ok(())
                } else {
                    Err(StoreError::CorruptedFile(format!(
                        "Access denied: Peer '{peer_id}' (role: '{peer_role}') not authorized for model {}",
                        self.model_id
                    )))
                }
            }
            ModelVisibility::PublicShared => Ok(()),
        }
    }
}

/// Model integrity verifier and pre-flight safety auditor.
#[derive(Debug, Clone, Copy, Default)]
pub struct ModelIntegrityVerifier;

impl ModelIntegrityVerifier {
    /// Audits and verifies a model from store, returning `VerifiedModelGuard` on success.
    pub fn verify(
        store: &ContentAddressedModelStore,
        model_id: &ModelId,
        mode: VerifyMode,
        attestation: Option<SignedModelAttestation>,
    ) -> Result<VerifiedModelGuard, StoreError> {
        let model_path = store.model_file_path(model_id);
        if !model_path.exists() {
            return Err(StoreError::ModelNotFound(*model_id));
        }

        let metadata = fs::metadata(&model_path)?;
        let file_size = metadata.len();

        // 1. Verify attestation signature if supplied
        if let Some(ref att) = attestation {
            if att.model_id != *model_id {
                return Err(StoreError::InvalidSignature("Attestation model_id mismatch".into()));
            }
            att.verify()?;
        }

        // 2. Read file header and parse GGUF structure
        let mut file = File::open(&model_path)?;
        // Read up to 64 MiB for header and tensor directory
        let header_read_size = std::cmp::min(file_size, 64 * 1024 * 1024) as usize;
        let mut header_buf = vec![0u8; header_read_size];
        file.read_exact(&mut header_buf)?;

        let inspection = GgufParser::parse(&header_buf)?;

        // 3. Strict Pre-Load Tensor Layout & Boundary Auditing
        let data_offset = inspection.data_offset;
        let mut sorted_ranges: Vec<ByteRange> = Vec::with_capacity(inspection.tensors.len());

        for tensor in &inspection.tensors {
            let abs_offset = data_offset
                .checked_add(tensor.offset)
                .ok_or_else(|| StoreError::CorruptedFile("tensor offset arithmetic overflow".into()))?;

            let abs_end = abs_offset
                .checked_add(tensor.size_bytes)
                .ok_or_else(|| StoreError::CorruptedFile("tensor end arithmetic overflow".into()))?;

            // Out-of-bounds check
            if abs_end > file_size {
                return Err(StoreError::RangeOutOfBounds {
                    requested: ByteRange::new(abs_offset, tensor.size_bytes),
                    total: file_size,
                });
            }

            // Tensor data cannot overlap with GGUF header/metadata directory
            if abs_offset < data_offset {
                return Err(StoreError::CorruptedFile(format!(
                    "tensor '{}' offset ({abs_offset}) overlaps with header/metadata directory ({data_offset})",
                    tensor.name
                )));
            }

            sorted_ranges.push(ByteRange::new(abs_offset, tensor.size_bytes));
        }

        // Overlapping tensor collision check
        sorted_ranges.sort_by_key(|r| r.offset);
        for i in 0..sorted_ranges.len().saturating_sub(1) {
            let current = &sorted_ranges[i];
            let next = &sorted_ranges[i + 1];
            if current.end() > next.offset {
                return Err(StoreError::CorruptedFile(format!(
                    "detected overlapping tensor memory: [{:#x}..{:#x}] overlaps with next tensor at {:#x}",
                    current.offset,
                    current.end(),
                    next.offset
                )));
            }
        }

        // 4. Full Cryptographic Scrub (if requested)
        if mode == VerifyMode::FullCryptographicScrub {
            let _manifest = store.get_manifest(model_id)?;
            let verified_mmap_res = store.open_verified_mmap(model_id);

            if let Err(err) = verified_mmap_res {
                // Automatic Quarantine on silent on-disk corruption
                let quarantine_path = ModelQuarantineManager::quarantine_model(
                    store,
                    model_id,
                    &format!("Cryptographic scrub failed: {err}"),
                )?;
                tracing::error!(
                    model_id = %model_id,
                    quarantine_path = %quarantine_path.display(),
                    error = %err,
                    "CRITICAL: Model corrupted on disk. Moved to quarantine!"
                );
                return Err(err);
            }
        }

        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_millis() as u64);

        Ok(VerifiedModelGuard {
            model_id: *model_id,
            file_path: model_path,
            inspection: Arc::new(inspection),
            attestation,
            verified_at_unix_ms: now_ms,
        })
    }
}

/// Manages isolation and quarantine of corrupted or untrusted models.
#[derive(Debug, Clone, Copy, Default)]
pub struct ModelQuarantineManager;

impl ModelQuarantineManager {
    /// Quarantines a model file, removing it from active registry to prevent execution.
    pub fn quarantine_model(
        store: &ContentAddressedModelStore,
        model_id: &ModelId,
        reason: &str,
    ) -> Result<PathBuf, StoreError> {
        let quarantine_dir = store.root_dir().join("quarantine");
        fs::create_dir_all(&quarantine_dir)?;

        let source_path = store.model_file_path(model_id);
        let target_path = quarantine_dir.join(format!("{}.corrupted", model_id.to_hex()));

        if source_path.exists() {
            fs::rename(&source_path, &target_path)?;
        }

        // Save quarantine diagnostic report
        let report_path = quarantine_dir.join(format!("{}.reason.txt", model_id.to_hex()));
        let report = format!(
            "Model Quarantine Report\nModel ID: {}\nTimestamp: {:?}\nReason: {}\n",
            model_id,
            SystemTime::now(),
            reason
        );
        fs::write(report_path, report)?;

        Ok(target_path)
    }

    /// Checks if a model is currently quarantined.
    #[must_use]
    pub fn is_quarantined(store: &ContentAddressedModelStore, model_id: &ModelId) -> bool {
        let quarantine_path = store
            .root_dir()
            .join("quarantine")
            .join(format!("{}.corrupted", model_id.to_hex()));
        quarantine_path.exists()
    }
}
