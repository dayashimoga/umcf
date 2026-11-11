//! Structured error types for Fuzzing and Parser Safety Verification.

use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum FuzzError {
    #[error("Pre-allocation limit exceeded for '{target}': requested {requested_bytes} bytes exceeds safety ceiling of {limit_bytes} bytes")]
    AllocationExceeded {
        target: String,
        requested_bytes: usize,
        limit_bytes: usize,
    },

    #[error("Recursion depth limit exceeded for '{target}': depth {depth} exceeds ceiling of {limit}")]
    RecursionDepthExceeded {
        target: String,
        depth: usize,
        limit: usize,
    },

    #[error("Malformed input rejected cleanly for target '{target}': {reason}")]
    MalformedInput {
        target: String,
        reason: String,
    },

    #[error("Panic or unexpected crash trapped for target '{target}': {reason}")]
    PanicTrapped {
        target: String,
        reason: String,
    },

    #[error("Timeout: processing input for target '{target}' took longer than 1.0 second SLA")]
    ProcessingTimeout {
        target: String,
    },
}

