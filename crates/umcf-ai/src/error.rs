//! Exhaustive domain errors for the UMCF AI inference engine.

use thiserror::Error;

/// Structured, actionable errors emitted by the inference engine and FFI bridge.
#[derive(Debug, Error)]
pub enum EngineError {
    #[error("failed to load model from '{path}': {reason}")]
    ModelLoadFailed { path: String, reason: String },

    #[error("model loading was cancelled by progress callback")]
    LoadAborted,

    #[error("failed to create inference context: {reason}")]
    ContextCreationFailed { reason: String },

    #[error("tokenization failed for text '{text}': {reason}")]
    TokenizeFailed { text: String, reason: String },

    #[error("inference decoding kernel failed with exit code {code}: {reason}")]
    DecodeFailed { code: i32, reason: String },

    #[error("inference decoding was aborted by user or timeout")]
    GenerationAborted,

    #[error("batch allocation or index error: {reason}")]
    InvalidBatch { reason: String },

    #[error("backend plugin signature mismatch for '{path}': expected BLAKE3 {expected}, got {actual}")]
    BackendPluginSignatureMismatch {
        path: String,
        expected: String,
        actual: String,
    },

    #[error("backend plugin load failed for '{path}': {reason}")]
    BackendPluginLoadFailed { path: String, reason: String },

    #[error("unverified model access: {reason}")]
    UnverifiedModel { reason: String },

    #[error("invalid inference input: {reason}")]
    InvalidInput { reason: String },

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}
