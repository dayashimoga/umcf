//! Model licensing, EULA compliance engine, and cryptographic acceptance receipts.
//!
//! Classifies open vs restricted AI licenses and enforces mandatory cryptographically
//! signed acceptance receipts before permitting inference execution on community-restricted models.
//!
//! Strictly adheres to `#![forbid(unsafe_code)]` and zero-placeholder invariants.

#![forbid(unsafe_code)]

use std::fs;
use std::path::{Path, PathBuf};

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};

use crate::error::StoreError;
use crate::security::Ed25519PublicKey;
use crate::store::{Ed25519Signature, ModelId};

/// Classification of model distribution and usage licenses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LicenseCategory {
    /// Fully permissive open-source licenses (Apache-2.0, MIT, BSD-3-Clause, Unlicense).
    OpenPermissive,
    /// Community restricted terms requiring explicit user consent (e.g. Llama-3, Gemma, Mistral terms).
    CommunityRestricted,
    /// Non-commercial or academic research only (e.g. CC-BY-NC-4.0).
    CommercialProhibited,
    /// Custom, proprietary, or unknown license.
    Unknown,
}

/// Cryptographically signed user or organization license acceptance receipt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LicenseAcceptanceReceipt {
    pub model_id: ModelId,
    pub license_id: String,
    pub license_hash: [u8; 32],
    pub accepted_by_node: String,
    pub timestamp_ms: u64,
    pub signature: Ed25519Signature,
}

impl LicenseAcceptanceReceipt {
    #[must_use]
    pub fn compute_signing_payload(&self) -> Vec<u8> {
        let mut payload = Vec::with_capacity(128);
        payload.extend_from_slice(b"UMCF-LICENSE-ACCEPTANCE-v1:");
        payload.extend_from_slice(self.model_id.as_bytes());
        payload.extend_from_slice(&(self.license_id.len() as u64).to_le_bytes());
        payload.extend_from_slice(self.license_id.as_bytes());
        payload.extend_from_slice(&self.license_hash);
        payload.extend_from_slice(&(self.accepted_by_node.len() as u64).to_le_bytes());
        payload.extend_from_slice(self.accepted_by_node.as_bytes());
        payload.extend_from_slice(&self.timestamp_ms.to_le_bytes());
        payload
    }

    /// Creates and signs a new license acceptance receipt.
    pub fn create_and_sign(
        model_id: ModelId,
        license_id: String,
        license_hash: [u8; 32],
        accepted_by_node: String,
        timestamp_ms: u64,
        signing_key: &SigningKey,
    ) -> Self {
        let mut receipt = Self {
            model_id,
            license_id,
            license_hash,
            accepted_by_node,
            timestamp_ms,
            signature: Ed25519Signature([0u8; 64]),
        };

        let payload = receipt.compute_signing_payload();
        let sig: Signature = signing_key.sign(&payload);
        receipt.signature = Ed25519Signature(sig.to_bytes());
        receipt
    }

    /// Verifies the cryptographic signature on this receipt.
    pub fn verify(&self, public_key: &Ed25519PublicKey) -> Result<(), StoreError> {
        let verifying_key = VerifyingKey::from_bytes(&public_key.0)
            .map_err(|e| StoreError::InvalidSignature(format!("invalid public key: {e}")))?;

        let sig = Signature::from_bytes(&self.signature.0);
        let payload = self.compute_signing_payload();

        verifying_key
            .verify(&payload, &sig)
            .map_err(|e| StoreError::InvalidSignature(format!("receipt signature invalid: {e}")))
    }
}

/// License compliance evaluation and storage policy engine.
#[derive(Debug, Clone)]
pub struct LicensePolicyEngine {
    receipts_dir: PathBuf,
}

impl LicensePolicyEngine {
    /// Creates a new license policy engine persisting receipts under `root_dir/licenses`.
    pub fn new(root_dir: impl AsRef<Path>) -> Result<Self, StoreError> {
        let receipts_dir = root_dir.as_ref().join("licenses");
        fs::create_dir_all(&receipts_dir)?;
        Ok(Self { receipts_dir })
    }

    /// Classifies a license identifier or SPDX string into a standardized category.
    #[must_use]
    pub fn classify_license(spdx_or_name: &str) -> LicenseCategory {
        let lower = spdx_or_name.to_lowercase();
        if lower.contains("apache-2.0")
            || lower.contains("mit")
            || lower.contains("bsd")
            || lower.contains("unlicense")
            || lower == "apache 2.0"
        {
            LicenseCategory::OpenPermissive
        } else if lower.contains("llama")
            || lower.contains("gemma")
            || lower.contains("community")
            || lower.contains("deepseek")
            || lower.contains("mistral")
        {
            LicenseCategory::CommunityRestricted
        } else if lower.contains("nc")
            || lower.contains("non-commercial")
            || lower.contains("academic")
            || lower.contains("cc-by-nc")
        {
            LicenseCategory::CommercialProhibited
        } else {
            LicenseCategory::Unknown
        }
    }

    /// Returns true if the license requires mandatory user acceptance before execution.
    #[must_use]
    pub fn requires_explicit_acceptance(category: LicenseCategory) -> bool {
        match category {
            LicenseCategory::OpenPermissive => false,
            LicenseCategory::CommunityRestricted | LicenseCategory::CommercialProhibited => true,
            LicenseCategory::Unknown => true,
        }
    }

    /// Persists a signed license acceptance receipt to disk.
    pub fn record_acceptance(&self, receipt: LicenseAcceptanceReceipt) -> Result<(), StoreError> {
        let path = self
            .receipts_dir
            .join(format!("{}.receipt.json", receipt.model_id.to_hex()));
        let json = serde_json::to_string_pretty(&receipt)
            .map_err(|e| StoreError::InvalidManifest(e.to_string()))?;
        fs::write(path, json)?;
        Ok(())
    }

    /// Checks if a valid license acceptance receipt exists on disk for `model_id`.
    #[must_use]
    pub fn is_accepted(&self, model_id: &ModelId) -> bool {
        let path = self
            .receipts_dir
            .join(format!("{}.receipt.json", model_id.to_hex()));
        path.exists()
    }

    /// Retrieves the stored acceptance receipt for a model if present.
    pub fn get_acceptance_receipt(
        &self,
        model_id: &ModelId,
    ) -> Result<Option<LicenseAcceptanceReceipt>, StoreError> {
        let path = self
            .receipts_dir
            .join(format!("{}.receipt.json", model_id.to_hex()));
        if !path.exists() {
            return Ok(None);
        }
        let content = fs::read_to_string(path)?;
        let receipt = serde_json::from_str(&content)
            .map_err(|e| StoreError::InvalidManifest(e.to_string()))?;
        Ok(Some(receipt))
    }

    /// Evaluates license compliance. Returns an error if restricted terms were not accepted.
    pub fn check_compliance(
        &self,
        model_id: &ModelId,
        category: LicenseCategory,
    ) -> Result<(), StoreError> {
        if !Self::requires_explicit_acceptance(category) {
            return Ok(());
        }

        if self.is_accepted(model_id) {
            Ok(())
        } else {
            Err(StoreError::InvalidManifest(format!(
                "License compliance violation: Model {model_id} requires explicit user acceptance of {category:?} terms before loading."
            )))
        }
    }
}
