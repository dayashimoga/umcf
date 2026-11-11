//! Structured, actionable domain errors for the UMCF multimedia and transcoding engine.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum MediaError {
    #[error("unsupported media codec: {codec}")]
    UnsupportedCodec { codec: String },

    #[error("hardware accelerator initialization failed for {backend}: {reason}")]
    HardwareInitializationFailed { backend: String, reason: String },

    #[error("transcode execution failed: {reason} (exit code: {exit_code:?})")]
    TranscodeExecutionFailed {
        reason: String,
        exit_code: Option<i32>,
    },

    #[error("transcode execution timed out after {elapsed_ms} ms")]
    TranscodeTimeout { elapsed_ms: u64 },

    #[error("hardware concurrent session limit exceeded: max allowed {max_sessions}")]
    SessionLimitExceeded { max_sessions: u8 },

    #[error("transcode cancellation timed out")]
    CancellationTimeout,

    #[error("invalid media argument or path: {reason}")]
    InvalidInput { reason: String },

    #[error("quality check failed: {metric} measured {measured:.4}, required {threshold:.4}")]
    QualityCheckFailed {
        metric: String,
        measured: f64,
        threshold: f64,
    },

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}
