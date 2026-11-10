//! `umcf-worker::error`: Structured errors and standard error codes for worker sandbox and execution.
//! Enforces `#![forbid(unsafe_code)]` and zero placeholders.

#![forbid(unsafe_code)]

use thiserror::Error;

/// Standard machine-readable error codes matching UMCF worker specifications.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[allow(non_camel_case_types)]
pub enum WorkerErrorCode {
    /// Input or output artifact BLAKE3 root hash mismatch (E4201).
    E4201_ARTIFACT_HASH_MISMATCH,
    /// Unauthorized sandbox filesystem path traversal or symlink escape attempt (E4202).
    E4202_SANDBOX_PATH_TRAVERSAL,
    /// Sandbox fuel exhaustion or memory allocation limit exceeded (E4203).
    E4203_SANDBOX_RESOURCE_EXHAUSTED,
    /// Native engine kernel not present in signed engine allow-list (E4204).
    E4204_UNAUTHORIZED_NATIVE_KERNEL,
    /// Task execution failed with non-zero exit code (E4205).
    E4205_TASK_EXECUTION_FAILED,
    /// Worker heartbeat timeout or node failure (E4206).
    E4206_HEARTBEAT_TIMEOUT,
    /// Intermediate checkpoint corrupted or invalid (E4207).
    E4207_CHECKPOINT_CORRUPTED,
}

impl std::fmt::Display for WorkerErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::E4201_ARTIFACT_HASH_MISMATCH => write!(f, "E4201_ARTIFACT_HASH_MISMATCH"),
            Self::E4202_SANDBOX_PATH_TRAVERSAL => write!(f, "E4202_SANDBOX_PATH_TRAVERSAL"),
            Self::E4203_SANDBOX_RESOURCE_EXHAUSTED => write!(f, "E4203_SANDBOX_RESOURCE_EXHAUSTED"),
            Self::E4204_UNAUTHORIZED_NATIVE_KERNEL => write!(f, "E4204_UNAUTHORIZED_NATIVE_KERNEL"),
            Self::E4205_TASK_EXECUTION_FAILED => write!(f, "E4205_TASK_EXECUTION_FAILED"),
            Self::E4206_HEARTBEAT_TIMEOUT => write!(f, "E4206_HEARTBEAT_TIMEOUT"),
            Self::E4207_CHECKPOINT_CORRUPTED => write!(f, "E4207_CHECKPOINT_CORRUPTED"),
        }
    }
}

/// Structured worker execution and sandbox error type.
#[derive(Debug, Error, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum WorkerError {
    #[error("E4201: Artifact BLAKE3 hash mismatch: expected {expected}, actual {actual}")]
    ArtifactHashMismatch { expected: String, actual: String },

    #[error("E4202: Sandbox path traversal attempt blocked for '{attempted_path}': {reason}")]
    SandboxPathTraversal {
        attempted_path: String,
        reason: String,
    },

    #[error("E4203: Sandbox resource exhausted: {reason}")]
    SandboxResourceExhausted { reason: String },

    #[error("E4204: Unauthorized native kernel '{engine_id}' (hash: {hash}) not in engine allow-list")]
    UnauthorizedNativeKernel { engine_id: String, hash: String },

    #[error("E4205: Task execution failed with exit code {exit_code}: {stderr}")]
    TaskExecutionFailed { exit_code: i32, stderr: String },

    #[error("E4206: Worker heartbeat timeout for node '{node_id}' (phi: {phi:.2})")]
    HeartbeatTimeout { node_id: String, phi: f64 },

    #[error("E4207: Task checkpoint corrupted: {0}")]
    CheckpointCorrupted(String),

    #[error("Worker task preempted by interactive user activity or thermal guard")]
    Preempted,

    #[error("I/O error during sandbox setup: {0}")]
    Io(String),
}

impl WorkerError {
    #[must_use]
    pub fn error_code(&self) -> WorkerErrorCode {
        match self {
            Self::ArtifactHashMismatch { .. } => WorkerErrorCode::E4201_ARTIFACT_HASH_MISMATCH,
            Self::SandboxPathTraversal { .. } => WorkerErrorCode::E4202_SANDBOX_PATH_TRAVERSAL,
            Self::SandboxResourceExhausted { .. } => {
                WorkerErrorCode::E4203_SANDBOX_RESOURCE_EXHAUSTED
            }
            Self::UnauthorizedNativeKernel { .. } => {
                WorkerErrorCode::E4204_UNAUTHORIZED_NATIVE_KERNEL
            }
            Self::TaskExecutionFailed { .. } => WorkerErrorCode::E4205_TASK_EXECUTION_FAILED,
            Self::HeartbeatTimeout { .. } => WorkerErrorCode::E4206_HEARTBEAT_TIMEOUT,
            Self::CheckpointCorrupted(_) => WorkerErrorCode::E4207_CHECKPOINT_CORRUPTED,
            Self::Preempted | Self::Io(_) => WorkerErrorCode::E4205_TASK_EXECUTION_FAILED,
        }
    }

    #[must_use]
    pub fn code(&self) -> WorkerErrorCode {
        self.error_code()
    }
}
