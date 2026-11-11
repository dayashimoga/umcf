//! Structured, actionable domain errors for the UMCF GPU compute and rendering engine.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum GpuError {
    #[error("shader validation error: {reason}")]
    ShaderValidationError { reason: String },

    #[error("shader resource limit exceeded: {limit} requested {requested}, maximum allowed {max_allowed}")]
    ShaderResourceLimitExceeded {
        limit: String,
        requested: usize,
        max_allowed: usize,
    },

    #[error("shader loop watchdog triggered: fuel exhausted after {max_iterations} iterations")]
    ShaderLoopFuelExhausted { max_iterations: u32 },

    #[error("out-of-bounds buffer access: offset {offset} + size {size} exceeds buffer length {buffer_len}")]
    OutOfBoundsBufferAccess {
        offset: usize,
        size: usize,
        buffer_len: usize,
    },

    #[error("VRAM allocation quota exceeded: requested {requested_bytes} bytes, available {available_bytes} bytes, limit {limit_bytes} bytes")]
    VramQuotaExceeded {
        requested_bytes: u64,
        available_bytes: u64,
        limit_bytes: u64,
    },

    #[error("GPU buffer with ID {buffer_id} not found in allocator registry")]
    BufferNotFound { buffer_id: u32 },

    #[error("GPU compute dispatch timed out after {elapsed_ms} ms")]
    DispatchTimeout { elapsed_ms: u64 },

    #[error("GPU device or adapter unavailable: {reason}")]
    DeviceUnavailable { reason: String },

    #[error("invalid input: {reason}")]
    InvalidInput { reason: String },

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}
