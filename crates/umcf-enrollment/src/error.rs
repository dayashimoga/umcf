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

    #[error(
        "requested token TTL of {requested_ms} ms exceeds maximum allowed ceiling of {max_ms} ms"
    )]
    MaxTtlExceeded { requested_ms: u64, max_ms: u64 },

    #[error("QR code matrix generation failed: {0}")]
    QrGenerationFailed(String),

    #[error("enrollment handshake failed: {0}")]
    HandshakeFailed(String),

    #[error("identity or certificate error during enrollment: {0}")]
    IdentityError(String),

    #[error("serialization error: {0}")]
    SerializationError(String),

    #[error("operation denied by RBAC policy: actor {actor_did} with role {role} lacks permission {permission} on scope {resource_scope}")]
    RbacDenied {
        actor_did: String,
        role: String,
        permission: String,
        resource_scope: String,
    },

    #[error("local resource-sharing consent violation: {reason}")]
    ConsentViolation { reason: String },

    #[error(
        "resource consent policy signature is invalid or not signed by device owner {owner_did}"
    )]
    ForgedConsentPolicy { owner_did: String },

    #[error("requested workload kind '{workload_kind}' is not permitted by node consent policy")]
    WorkloadNotPermitted { workload_kind: String },

    #[error("invalid role transition from {from} to {to}: {reason}")]
    InvalidRoleTransition {
        from: String,
        to: String,
        reason: String,
    },

    #[error("leader election failed: {0}")]
    ElectionFailed(String),
}

impl EnrollmentError {
    /// Maps the enrollment error to standard UMCF ErrorCode taxonomy.
    #[must_use]
    pub const fn error_code(&self) -> ErrorCode {
        match self {
            Self::TokenExpired { .. } | Self::MaxTtlExceeded { .. } => ErrorCode::E2003TokenExpired,
            Self::TokenReplayed(_) | Self::InvalidSignature | Self::ForgedConsentPolicy { .. } => {
                ErrorCode::E2002InvalidSignature
            }
            Self::MalformedUri(_) => ErrorCode::E2001InvalidNodeId,
            Self::RbacDenied { .. } | Self::InvalidRoleTransition { .. } => {
                ErrorCode::E2004UnauthorizedCapability
            }
            Self::ConsentViolation { .. } | Self::WorkloadNotPermitted { .. } => {
                ErrorCode::E4004ConsentDenied
            }
            Self::HandshakeFailed(_) | Self::ElectionFailed(_) => ErrorCode::E3001ConnectionFailed,
            Self::QrGenerationFailed(_) | Self::SerializationError(_) | Self::IdentityError(_) => {
                ErrorCode::E8002UnhandledInternalError
            }
        }
    }
}
