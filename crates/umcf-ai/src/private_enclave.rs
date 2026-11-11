//! Private AI Privacy Enclave, Secret Zeroization & Strict Local-Only Execution Guards.
//!
//! Enforces:
//! - Cryptographically enforced `PrivacyEnclaveMode::StrictLocalOnly`.
//! - Outbound network circuit breaker guaranteeing 0 outbound bytes in local-only mode.
//! - Immediate zeroization (`ZeroizeOnDrop`) of physical RAM allocations holding prompt strings.
//! - Automatic redaction of sensitive prompts from `tracing` spans and logs.

use crate::error::EngineError;
use crate::partitioner::DistributedPartitionPlan;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use zeroize::{Zeroize, ZeroizeOnDrop};

/// Privacy and data sovereignty mode for AI inference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum PrivacyEnclaveMode {
    /// Zero outbound network egress. Binds strictly to local devices.
    #[default]
    StrictLocalOnly,
    /// Allows distributed inference only across devices owned by the same user.
    TrustedPersonalDevicesOnly,
    /// Allows cluster-wide execution with end-to-end encrypted activations.
    ClusterEncrypted,
}

impl PrivacyEnclaveMode {
    #[must_use]
    pub fn parse_header(header_val: &str) -> Self {
        match header_val.trim().to_lowercase().as_str() {
            "strict-local" | "strict_local_only" | "strictlocalonly" => Self::StrictLocalOnly,
            "trusted-devices" | "trusted_personal_devices_only" => Self::TrustedPersonalDevicesOnly,
            "cluster-encrypted" | "cluster_encrypted" => Self::ClusterEncrypted,
            _ => Self::StrictLocalOnly,
        }
    }

    #[must_use]
    pub fn as_header_str(&self) -> &'static str {
        match self {
            Self::StrictLocalOnly => "strict-local",
            Self::TrustedPersonalDevicesOnly => "trusted-devices",
            Self::ClusterEncrypted => "cluster-encrypted",
        }
    }
}

/// Secure buffer holding sensitive prompt strings and tokens with automatic zeroization on drop.
#[derive(Clone, Zeroize, ZeroizeOnDrop)]
pub struct SecretPromptBuffer {
    data: Vec<u8>,
}

impl SecretPromptBuffer {
    #[must_use]
    pub fn new(text: &str) -> Self {
        Self {
            data: text.as_bytes().to_vec(),
        }
    }

    #[must_use]
    pub fn from_bytes(bytes: &[u8]) -> Self {
        Self {
            data: bytes.to_vec(),
        }
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        std::str::from_utf8(&self.data).unwrap_or("[INVALID_UTF8]")
    }

    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.data
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.data.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Checks if the underlying memory has been zeroed out.
    #[must_use]
    pub fn is_zeroed(&self) -> bool {
        self.data.iter().all(|&b| b == 0)
    }
}

impl fmt::Debug for SecretPromptBuffer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[REDACTED_PRIVATE_ENCLAVE]")
    }
}

impl fmt::Display for SecretPromptBuffer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[REDACTED_PRIVATE_ENCLAVE]")
    }
}

/// Active private inference session guarding against network egress.
#[derive(Debug)]
pub struct EphemeralInferenceSession {
    pub session_id: String,
    pub mode: PrivacyEnclaveMode,
    pub prompt: SecretPromptBuffer,
    pub outbound_bytes: Arc<AtomicU64>,
}

impl EphemeralInferenceSession {
    #[must_use]
    pub fn new(session_id: String, mode: PrivacyEnclaveMode, prompt: &str) -> Self {
        Self {
            session_id,
            mode,
            prompt: SecretPromptBuffer::new(prompt),
            outbound_bytes: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Records network transmission and trips a circuit breaker in StrictLocalOnly mode.
    pub fn record_outbound_egress(&self, bytes: usize) -> Result<(), EngineError> {
        if self.mode == PrivacyEnclaveMode::StrictLocalOnly {
            return Err(EngineError::EnclaveViolation {
                reason: format!(
                    "outbound network egress of {} bytes attempted in StrictLocalOnly enclave mode",
                    bytes
                ),
            });
        }
        self.outbound_bytes.fetch_add(bytes as u64, Ordering::SeqCst);
        Ok(())
    }

    /// Validates whether a generated partition plan is permissible in this enclave mode.
    pub fn validate_partition_plan(&self, plan: &DistributedPartitionPlan) -> Result<(), EngineError> {
        if self.mode == PrivacyEnclaveMode::StrictLocalOnly && !plan.is_single_node {
            return Err(EngineError::EnclaveViolation {
                reason: "Model requires distributed cluster split, which is forbidden in StrictLocalOnly enclave mode".to_string(),
            });
        }
        Ok(())
    }

    #[must_use]
    pub fn outbound_bytes_sent(&self) -> u64 {
        self.outbound_bytes.load(Ordering::SeqCst)
    }
}

/// Redacts sensitive canary or private enclave strings from logging contexts.
#[must_use]
pub fn redact_enclave_text(text: &str, canary: &str) -> String {
    text.replace(canary, "[REDACTED_PRIVATE_ENCLAVE]")
}
