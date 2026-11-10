//! Standardized error taxonomy, RFC-9457 Problem Details, and retryability classifications for UMCF.
//! Adheres strictly to `#![forbid(unsafe_code)]` and zero-placeholder invariants.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::time::Duration;
use thiserror::Error;

/// Standardized UMCF error codes across categories E1xxx to E8xxx.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ErrorCode {
    // E1xxx: Protocol
    E1001IncompatibleProtocolVersion,
    E1002PlaneIsolationViolation,
    E1003FrameTooLarge,
    E1004ChecksumMismatch,
    E1005MalformedWireFrame,

    // E2xxx: Identity & Auth
    E2001InvalidNodeId,
    E2002InvalidSignature,
    E2003TokenExpired,
    E2004UnauthorizedCapability,

    // E3xxx: Network
    E3001ConnectionFailed,
    E3002NetworkTimeout,
    E3003NatTraversalFailed,

    // E4xxx: Scheduler & Resource
    E4001InsufficientResources,
    E4002ThermalCutoff,
    E4003BatteryLow,
    E4004ConsentDenied,

    // E5xxx: Inference
    E5001ModelLoadFailed,
    E5002OutOfVram,
    E5003TokenGenerationFailed,

    // E6xxx: Media & GPU
    E6001BackendUnavailable,
    E6002FrameEncodeFailed,
    E6003DeviceLost,

    // E7xxx: Storage & Ledger
    E7001StorageQuotaExceeded,
    E7002ChunkNotFound,
    E7003LedgerIntegrityFailed,

    // E8xxx: Internal
    E8001InternalPanic,
    E8002UnhandledInternalError,
}

impl ErrorCode {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::E1001IncompatibleProtocolVersion => "E1001",
            Self::E1002PlaneIsolationViolation => "E1002",
            Self::E1003FrameTooLarge => "E1003",
            Self::E1004ChecksumMismatch => "E1004",
            Self::E1005MalformedWireFrame => "E1005",
            Self::E2001InvalidNodeId => "E2001",
            Self::E2002InvalidSignature => "E2002",
            Self::E2003TokenExpired => "E2003",
            Self::E2004UnauthorizedCapability => "E2004",
            Self::E3001ConnectionFailed => "E3001",
            Self::E3002NetworkTimeout => "E3002",
            Self::E3003NatTraversalFailed => "E3003",
            Self::E4001InsufficientResources => "E4001",
            Self::E4002ThermalCutoff => "E4002",
            Self::E4003BatteryLow => "E4003",
            Self::E4004ConsentDenied => "E4004",
            Self::E5001ModelLoadFailed => "E5001",
            Self::E5002OutOfVram => "E5002",
            Self::E5003TokenGenerationFailed => "E5003",
            Self::E6001BackendUnavailable => "E6001",
            Self::E6002FrameEncodeFailed => "E6002",
            Self::E6003DeviceLost => "E6003",
            Self::E7001StorageQuotaExceeded => "E7001",
            Self::E7002ChunkNotFound => "E7002",
            Self::E7003LedgerIntegrityFailed => "E7003",
            Self::E8001InternalPanic => "E8001",
            Self::E8002UnhandledInternalError => "E8002",
        }
    }

    #[must_use]
    pub const fn title(self) -> &'static str {
        match self {
            Self::E1001IncompatibleProtocolVersion => "Incompatible Protocol Version",
            Self::E1002PlaneIsolationViolation => "Plane Isolation Violation",
            Self::E1003FrameTooLarge => "Frame Exceeds Maximum Size Limit",
            Self::E1004ChecksumMismatch => "Frame Integrity Checksum Mismatch",
            Self::E1005MalformedWireFrame => "Malformed Wire Frame Header or Structure",
            Self::E2001InvalidNodeId => "Invalid Node Identifier Format",
            Self::E2002InvalidSignature => "Cryptographic Signature Verification Failed",
            Self::E2003TokenExpired => "Security or Enrollment Token Expired",
            Self::E2004UnauthorizedCapability => "Action Not Permitted by Capability Token",
            Self::E3001ConnectionFailed => "P2P or Control Plane Connection Failed",
            Self::E3002NetworkTimeout => "Operation Timed Out Over Transport",
            Self::E3003NatTraversalFailed => "STUN/TURN NAT Traversal Failed",
            Self::E4001InsufficientResources => "Insufficient Hardware Capacity for Task",
            Self::E4002ThermalCutoff => "Thermal Headroom Exceeded Safe Ceiling",
            Self::E4003BatteryLow => "Battery Level Below User Configured Policy Floor",
            Self::E4004ConsentDenied => "Explicit User Resource-Sharing Consent Denied",
            Self::E5001ModelLoadFailed => "Model Loading or Verification Failed",
            Self::E5002OutOfVram => "Insufficient Physical VRAM for Model Shard",
            Self::E5003TokenGenerationFailed => "Inference Token Generation Failed",
            Self::E6001BackendUnavailable => "Requested Hardware Accelerator Backend Unavailable",
            Self::E6002FrameEncodeFailed => "Hardware Video Frame Encoding Failed",
            Self::E6003DeviceLost => "GPU or Accelerator Driver Device Lost",
            Self::E7001StorageQuotaExceeded => "Local Storage Quota Exceeded",
            Self::E7002ChunkNotFound => "Requested Content-Addressed Chunk Not Found",
            Self::E7003LedgerIntegrityFailed => "Accounting Ledger Integrity Check Failed",
            Self::E8001InternalPanic => "Internal Runtime Invariant Violation",
            Self::E8002UnhandledInternalError => "Unhandled Internal Error",
        }
    }

    #[must_use]
    pub const fn http_status(self) -> u16 {
        match self {
            Self::E1001IncompatibleProtocolVersion => 426, // Upgrade Required
            Self::E1002PlaneIsolationViolation => 422,     // Unprocessable Entity
            Self::E1003FrameTooLarge => 413,               // Content Too Large
            Self::E1004ChecksumMismatch | Self::E1005MalformedWireFrame => 400, // Bad Request
            Self::E2001InvalidNodeId | Self::E2002InvalidSignature => 401, // Unauthorized
            Self::E2003TokenExpired => 401,
            Self::E2004UnauthorizedCapability => 403, // Forbidden
            Self::E3001ConnectionFailed => 502,       // Bad Gateway
            Self::E3002NetworkTimeout => 504,         // Gateway Timeout
            Self::E3003NatTraversalFailed => 502,
            Self::E4001InsufficientResources => 507, // Insufficient Storage
            Self::E4002ThermalCutoff | Self::E4003BatteryLow | Self::E4004ConsentDenied => 503, // Service Unavailable
            Self::E5001ModelLoadFailed
            | Self::E5002OutOfVram
            | Self::E5003TokenGenerationFailed => 500,
            Self::E6001BackendUnavailable
            | Self::E6002FrameEncodeFailed
            | Self::E6003DeviceLost => 503,
            Self::E7001StorageQuotaExceeded => 507,
            Self::E7002ChunkNotFound => 404, // Not Found
            Self::E7003LedgerIntegrityFailed => 500,
            Self::E8001InternalPanic | Self::E8002UnhandledInternalError => 500,
        }
    }

    #[must_use]
    pub const fn grpc_status(self) -> u32 {
        match self {
            Self::E1001IncompatibleProtocolVersion => 9, // FAILED_PRECONDITION
            Self::E1002PlaneIsolationViolation => 3,     // INVALID_ARGUMENT
            Self::E1003FrameTooLarge => 11,              // OUT_OF_RANGE
            Self::E1004ChecksumMismatch | Self::E1005MalformedWireFrame => 3, // INVALID_ARGUMENT
            Self::E2001InvalidNodeId | Self::E2002InvalidSignature | Self::E2003TokenExpired => 16, // UNAUTHENTICATED
            Self::E2004UnauthorizedCapability => 7, // PERMISSION_DENIED
            Self::E3001ConnectionFailed => 14,      // UNAVAILABLE
            Self::E3002NetworkTimeout => 4,         // DEADLINE_EXCEEDED
            Self::E3003NatTraversalFailed => 14,
            Self::E4001InsufficientResources => 8, // RESOURCE_EXHAUSTED
            Self::E4002ThermalCutoff | Self::E4003BatteryLow | Self::E4004ConsentDenied => 14, // UNAVAILABLE
            Self::E5001ModelLoadFailed
            | Self::E5002OutOfVram
            | Self::E5003TokenGenerationFailed => 13, // INTERNAL
            Self::E6001BackendUnavailable
            | Self::E6002FrameEncodeFailed
            | Self::E6003DeviceLost => 14,
            Self::E7001StorageQuotaExceeded => 8, // RESOURCE_EXHAUSTED
            Self::E7002ChunkNotFound => 5,        // NOT_FOUND
            Self::E7003LedgerIntegrityFailed => 15, // DATA_LOSS
            Self::E8001InternalPanic | Self::E8002UnhandledInternalError => 13, // INTERNAL
        }
    }

    #[must_use]
    pub const fn retryability(self) -> Retryability {
        match self {
            Self::E1001IncompatibleProtocolVersion => Retryability::Fatal,
            Self::E1002PlaneIsolationViolation => Retryability::Fatal,
            Self::E1003FrameTooLarge => Retryability::Fatal,
            Self::E1004ChecksumMismatch => Retryability::Immediate,
            Self::E1005MalformedWireFrame => Retryability::Fatal,
            Self::E2001InvalidNodeId | Self::E2002InvalidSignature => Retryability::Fatal,
            Self::E2003TokenExpired => Retryability::Fatal,
            Self::E2004UnauthorizedCapability => Retryability::Fatal,
            Self::E3001ConnectionFailed => Retryability::After(Duration::from_millis(500)),
            Self::E3002NetworkTimeout => Retryability::Immediate,
            Self::E3003NatTraversalFailed => Retryability::After(Duration::from_secs(2)),
            Self::E4001InsufficientResources => Retryability::After(Duration::from_secs(5)),
            Self::E4002ThermalCutoff => Retryability::After(Duration::from_secs(10)),
            Self::E4003BatteryLow => Retryability::After(Duration::from_secs(30)),
            Self::E4004ConsentDenied => Retryability::Fatal,
            Self::E5001ModelLoadFailed => Retryability::Fatal,
            Self::E5002OutOfVram => Retryability::After(Duration::from_secs(2)),
            Self::E5003TokenGenerationFailed => Retryability::Immediate,
            Self::E6001BackendUnavailable => Retryability::Fatal,
            Self::E6002FrameEncodeFailed => Retryability::Immediate,
            Self::E6003DeviceLost => Retryability::After(Duration::from_secs(1)),
            Self::E7001StorageQuotaExceeded => Retryability::Fatal,
            Self::E7002ChunkNotFound => Retryability::Fatal,
            Self::E7003LedgerIntegrityFailed => Retryability::Fatal,
            Self::E8001InternalPanic | Self::E8002UnhandledInternalError => Retryability::Fatal,
        }
    }
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.as_str(), self.title())
    }
}

/// Actionable classification for retry handling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Retryability {
    Immediate,
    After(Duration),
    Fatal,
}

impl fmt::Display for Retryability {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Immediate => write!(f, "RetryableImmediate"),
            Self::After(d) => write!(f, "RetryableAfter({}ms)", d.as_millis()),
            Self::Fatal => write!(f, "Fatal"),
        }
    }
}

/// Standardized UMCF error type adhering to the global taxonomy.
#[derive(Debug, Clone, PartialEq, Eq, Error, Serialize, Deserialize)]
#[error("{code}: {message}")]
pub struct UmcfError {
    pub code: ErrorCode,
    pub message: String,
    pub detail: Option<String>,
    pub trace_id: Option<String>,
}

impl UmcfError {
    #[must_use]
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            detail: None,
            trace_id: None,
        }
    }

    #[must_use]
    pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    #[must_use]
    pub fn with_trace_id(mut self, trace_id: impl Into<String>) -> Self {
        self.trace_id = Some(trace_id.into());
        self
    }

    #[must_use]
    pub const fn code(&self) -> ErrorCode {
        self.code
    }

    #[must_use]
    pub const fn http_status(&self) -> u16 {
        self.code.http_status()
    }

    #[must_use]
    pub const fn grpc_status(&self) -> u32 {
        self.code.grpc_status()
    }

    #[must_use]
    pub const fn retryability(&self) -> Retryability {
        self.code.retryability()
    }

    /// Serializes error into standard RFC-9457 Problem Details JSON format.
    #[must_use]
    pub fn to_problem_details(&self) -> ProblemDetails {
        ProblemDetails {
            type_uri: format!("urn:umcf:error:{}", self.code.as_str()),
            title: self.code.title().to_string(),
            status: self.http_status(),
            detail: self.detail.clone().unwrap_or_else(|| self.message.clone()),
            instance: self
                .trace_id
                .as_ref()
                .map(|t| format!("urn:umcf:trace:{t}")),
            error_code: self.code.as_str().to_string(),
            retryability: self.retryability().to_string(),
        }
    }
}

/// RFC-9457 Problem Details object representation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProblemDetails {
    #[serde(rename = "type")]
    pub type_uri: String,
    pub title: String,
    pub status: u16,
    pub detail: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instance: Option<String>,
    pub error_code: String,
    pub retryability: String,
}

/// Item in the machine-readable error catalog.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ErrorCatalogEntry {
    pub code: String,
    pub title: String,
    pub http_status: u16,
    pub grpc_status: u32,
    pub retryability: String,
}

/// Generates the complete, exhaustive taxonomy catalog for documentation and tooling.
#[must_use]
pub fn all_error_taxonomy_entries() -> Vec<ErrorCatalogEntry> {
    let all_codes = [
        ErrorCode::E1001IncompatibleProtocolVersion,
        ErrorCode::E1002PlaneIsolationViolation,
        ErrorCode::E1003FrameTooLarge,
        ErrorCode::E1004ChecksumMismatch,
        ErrorCode::E1005MalformedWireFrame,
        ErrorCode::E2001InvalidNodeId,
        ErrorCode::E2002InvalidSignature,
        ErrorCode::E2003TokenExpired,
        ErrorCode::E2004UnauthorizedCapability,
        ErrorCode::E3001ConnectionFailed,
        ErrorCode::E3002NetworkTimeout,
        ErrorCode::E3003NatTraversalFailed,
        ErrorCode::E4001InsufficientResources,
        ErrorCode::E4002ThermalCutoff,
        ErrorCode::E4003BatteryLow,
        ErrorCode::E4004ConsentDenied,
        ErrorCode::E5001ModelLoadFailed,
        ErrorCode::E5002OutOfVram,
        ErrorCode::E5003TokenGenerationFailed,
        ErrorCode::E6001BackendUnavailable,
        ErrorCode::E6002FrameEncodeFailed,
        ErrorCode::E6003DeviceLost,
        ErrorCode::E7001StorageQuotaExceeded,
        ErrorCode::E7002ChunkNotFound,
        ErrorCode::E7003LedgerIntegrityFailed,
        ErrorCode::E8001InternalPanic,
        ErrorCode::E8002UnhandledInternalError,
    ];

    all_codes
        .into_iter()
        .map(|c| ErrorCatalogEntry {
            code: c.as_str().to_string(),
            title: c.title().to_string(),
            http_status: c.http_status(),
            grpc_status: c.grpc_status(),
            retryability: c.retryability().to_string(),
        })
        .collect()
}
