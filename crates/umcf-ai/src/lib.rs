//! Universal Mobile Compute Fabric (UMCF) AI Inference Engine.
//!
//! Provides safe RAII wrappers over `llama.cpp` and `ggml` backend dispatch.

pub mod backend_loader;
pub mod backend_selector;
pub mod error;
pub mod ffi_bridge;

pub use backend_loader::{DynamicBackendLoader, GgmlBackendDescriptor};
pub use backend_selector::{
    BackendExecutionPlan, BackendKind, CpuBackend, CpuIsaFlags, FallbackExecutor, FallbackTier,
    GpuBackend, HardwareBackendSelector,
};
pub use error::EngineError;
pub use ffi_bridge::{
    ContextParams, LlamaBackendGuard, LlamaBackendManager, ModelLoadParams, SafeLlamaBatch,
    SafeLlamaContext, SafeLlamaModel, SafeSamplerChain,
};
