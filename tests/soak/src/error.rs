//! Error types for Soak Testing and Resource Leak Profiling.

use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq)]
pub enum SoakError {
    #[error("Resource leak detected for metric '{metric}': regression slope is {slope:.4} (must be <= {threshold:.4}). Details: {details}")]
    LeakDetected {
        metric: String,
        slope: f64,
        threshold: f64,
        details: String,
    },

    #[error("Quiescence barrier timed out after {elapsed_ms} ms: {active_count} active resources remaining")]
    QuiescenceTimeout {
        elapsed_ms: u64,
        active_count: usize,
    },

    #[error("Handle count anomaly: expected <= {expected}, observed {actual}")]
    HandleCountAnomaly { expected: usize, actual: usize },

    #[error("Soak cycle {cycle} execution failed: {reason}")]
    SoakCycleFailed { cycle: usize, reason: String },

    #[error("OLS regression calculation error: {reason}")]
    RegressionError { reason: String },
}
