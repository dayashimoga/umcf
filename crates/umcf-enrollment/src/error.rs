//! Strongly typed errors for the UMCF device enrollment subsystem.
//! Strictly enforces `#![forbid(unsafe_code)]` and zero-placeholder invariants.

use thiserror::Error;
use umcf_telemetry::error::ErrorCode;

/// Strongly typed errors for UMCF enrollment, QR code rendering, and handshake verification.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum EnrollmentError {
    #[error("enrollment token has expired: expired at {expires_at_ms}, current {now_ms}")]
    TokenExpired { expires_at_ms: u64, now_ms: u64 },

    #[error("enrollment token has already been redeemed (replay detected: nonce {0:?})")]
    TokenReplayed([u8; 16]),

    #[error("cryptographic signature on enrollment token is invalid")]
    InvalidSignature,

    #[error("malformed enrollment URI or base64url payload: {0}")]
    MalformedUri(String),

    #[error("requested token TTL of {requested_ms} ms exceeds maximum allowed ceiling of {max_ms} ms")]
    MaxTtlExceeded { requested_ms: u64, max_ms: u64 },

    #[error("QR code matrix generation failed: {0}")]
    QrGenerationFailed(String),

    #[error("enrollment handshake failed: {0}")]
    HandshakeFailed(String),

    #[error("identity or certificate error during enrollment: {0}")]
    IdentityError(String),

    #[error("serialization error: {0}")]
    SerializationError(String),
}

impl EnrollmentError {
    /// Maps the enrollment error to standard UMCF ErrorCode taxonomy.
    #[must_use]
    pub const fn error_code(&self) -> ErrorCode {
        match self {
            Self::TokenExpired { .. } | Self::MaxTtlExceeded { .. } => ErrorCode::E2003TokenExpired,
            Self::TokenReplayed(_) | Self::InvalidSignature => ErrorCode::E2002InvalidSignature,
            Self::MalformedUri(_) => ErrorCode::E2001InvalidNodeId,
            Self::HandshakeFailed(_) => ErrorCode::E3001ConnectionFailed,
            Self::QrGenerationFailed(_) | Self::SerializationError(_) | Self::IdentityError(_) => {
                ErrorCode::E8002UnhandledInternalError
            }
        }
    }
}
