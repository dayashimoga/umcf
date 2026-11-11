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

    #[error("insufficient cluster-wide memory: required {required_bytes} bytes, available {available_bytes} bytes, deficit {deficit_bytes} bytes")]
    InsufficientClusterMemory {
        required_bytes: u64,
        available_bytes: u64,
        deficit_bytes: u64,
    },

    #[error("no eligible nodes available in cluster for partition: {reason}")]
    NoEligibleNodes { reason: String },

    #[error("invalid partition plan: {reason}")]
    InvalidPartitionPlan { reason: String },

    #[error("RPC authentication or capability token validation failed: {reason}")]
    RpcAuthenticationFailed { reason: String },

    #[error("RPC activation payload checksum mismatch: expected {expected}, got {actual}")]
    RpcChecksumMismatch { expected: String, actual: String },

    #[error("pipeline worker node {node_id} failed: {reason}")]
    WorkerNodeFailed { node_id: String, reason: String },

    #[error("pipeline failover timed out after {elapsed_ms} ms")]
    FailoverTimeout { elapsed_ms: u64 },

    #[error("private enclave security violation: {reason}")]
    EnclaveViolation { reason: String },

    #[error("chat template rendering failed: {reason}")]
    ChatTemplateError { reason: String },

    #[error("HTTP error {status_code}: {message}")]
    HttpError { status_code: u16, message: String },

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}
