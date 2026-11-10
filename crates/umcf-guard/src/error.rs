//! `umcf-guard::error`: Structured error types and standard error codes.
//! Strictly enforces `#![forbid(unsafe_code)]` and zero placeholders.

#![forbid(unsafe_code)]

/// Standard machine-readable error codes matching UMCF specifications.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[allow(non_camel_case_types)]
pub enum ErrorCode {
    /// Memory allocation quota exceeded (E4001).
    E4001_MEMORY_QUOTA_EXCEEDED,
    /// VRAM allocation quota exceeded (E4002).
    E4002_VRAM_QUOTA_EXCEEDED,
    /// CPU quota invalid or out of bounds (E4003).
    E4003_CPU_QUOTA_INVALID,
    /// Underlying platform backend failed (E4004).
    E4004_PLATFORM_BACKEND_ERROR,
    /// Referenced task was not found (E4005).
    E4005_TASK_NOT_FOUND,
    /// Resource reservation lease is invalid or expired (E4006).
    E4006_LEASE_INVALID,
    /// Resource consent policy rejected (E4007).
    E4007_POLICY_REJECTED,
}

impl std::fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::E4001_MEMORY_QUOTA_EXCEEDED => write!(f, "E4001_MEMORY_QUOTA_EXCEEDED"),
            Self::E4002_VRAM_QUOTA_EXCEEDED => write!(f, "E4002_VRAM_QUOTA_EXCEEDED"),
            Self::E4003_CPU_QUOTA_INVALID => write!(f, "E4003_CPU_QUOTA_INVALID"),
            Self::E4004_PLATFORM_BACKEND_ERROR => write!(f, "E4004_PLATFORM_BACKEND_ERROR"),
            Self::E4005_TASK_NOT_FOUND => write!(f, "E4005_TASK_NOT_FOUND"),
            Self::E4006_LEASE_INVALID => write!(f, "E4006_LEASE_INVALID"),
            Self::E4007_POLICY_REJECTED => write!(f, "E4007_POLICY_REJECTED"),
        }
    }
}

/// Structured error type for resource guard and quota enforcement operations.
#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum GuardError {
    #[error("E4001: Memory quota exceeded: requested {requested_bytes} bytes, available {available_bytes} bytes (ceiling: {ceiling_bytes} bytes)")]
    MemoryQuotaExceeded {
        requested_bytes: u64,
        available_bytes: u64,
        ceiling_bytes: u64,
    },

    #[error("E4002: VRAM quota exceeded: requested {requested_bytes} bytes, available {available_bytes} bytes (ceiling: {ceiling_bytes} bytes)")]
    VramQuotaExceeded {
        requested_bytes: u64,
        available_bytes: u64,
        ceiling_bytes: u64,
    },

    #[error("E4003: CPU quota invalid or out of bounds: {0}")]
    CpuQuotaInvalid(String),

    #[error("E4004: Platform quota backend error: {0}")]
    PlatformBackend(String),

    #[error("E4005: Task not found: {0}")]
    TaskNotFound(String),

    #[error("E4006: Lease invalid or already released: {0}")]
    LeaseInvalid(String),

    #[error("E4007: Resource consent policy rejected: {0}")]
    PolicyRejected(String),
}

impl GuardError {
    /// Maps this error variant to the standard `ErrorCode`.
    #[must_use]
    pub fn error_code(&self) -> ErrorCode {
        match self {
            Self::MemoryQuotaExceeded { .. } => ErrorCode::E4001_MEMORY_QUOTA_EXCEEDED,
            Self::VramQuotaExceeded { .. } => ErrorCode::E4002_VRAM_QUOTA_EXCEEDED,
            Self::CpuQuotaInvalid(_) => ErrorCode::E4003_CPU_QUOTA_INVALID,
            Self::PlatformBackend(_) => ErrorCode::E4004_PLATFORM_BACKEND_ERROR,
            Self::TaskNotFound(_) => ErrorCode::E4005_TASK_NOT_FOUND,
            Self::LeaseInvalid(_) => ErrorCode::E4006_LEASE_INVALID,
            Self::PolicyRejected(_) => ErrorCode::E4007_POLICY_REJECTED,
        }
    }
}

