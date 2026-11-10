//! Error definitions for `umcf-identity`.
//! Strictly adheres to `#![forbid(unsafe_code)]` and zero-placeholder invariants.

use thiserror::Error;
use umcf_telemetry::error::ErrorCode;

/// Strongly typed cryptographic and identity errors for UMCF.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum IdentityError {
    #[error("cryptographic signature is invalid or corrupted")]
    InvalidSignature,

    #[error("DID string format is malformed: {0}")]
    MalformedDid(String),

    #[error("unsupported DID method or multicodec prefix: {0}")]
    UnsupportedDidMethod(String),

    #[error("public key bytes invalid or point not on curve: {0}")]
    InvalidPublicKey(String),

    #[error("sender DID does not match signing key: expected {expected}, actual {actual}")]
    SenderMismatch { expected: String, actual: String },

    #[error("signature replay or envelope mutation detected")]
    EnvelopeTampered,

    #[error("JSON serialization failed: {0}")]
    SerializationError(String),
}

impl IdentityError {
    /// Maps the identity error to standard UMCF ErrorCode taxonomy.
    #[must_use]
    pub const fn error_code(&self) -> ErrorCode {
        match self {
            Self::InvalidSignature | Self::EnvelopeTampered => ErrorCode::E2002InvalidSignature,
            Self::MalformedDid(_)
            | Self::UnsupportedDidMethod(_)
            | Self::InvalidPublicKey(_)
            | Self::SenderMismatch { .. } => ErrorCode::E2001InvalidNodeId,
            Self::SerializationError(_) => ErrorCode::E8002UnhandledInternalError,
        }
    }
}
