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

    #[error("segment {segment_index} frame count mismatch: expected {expected}, actual {actual}")]
    SegmentFrameCountMismatch {
        segment_index: u32,
        expected: u64,
        actual: u64,
    },

    #[error("PTS/DTS discontinuity at segment {segment_index}: previous PTS {prev_pts}, current PTS {curr_pts}")]
    PtsDtsDiscontinuity {
        segment_index: u32,
        prev_pts: i64,
        curr_pts: i64,
    },

    #[error("audio/video synchronization drift exceeded: measured {measured_ms:.2} ms, maximum allowed {max_allowed_ms:.2} ms")]
    AvSyncDriftExceeded {
        measured_ms: f64,
        max_allowed_ms: f64,
    },

    #[error("quality check failed: {metric} measured {measured:.4}, required {threshold:.4}")]
    QualityCheckFailed {
        metric: String,
        measured: f64,
        threshold: f64,
    },

    #[error("segment {segment_index} failed all recovery retries (max {max_retries}): {last_reason}")]
    SegmentRecoveryExhausted {
        segment_index: u32,
        max_retries: u32,
        last_reason: String,
    },

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}
