//! Universal Mobile Compute Fabric (UMCF) AI Inference Engine.
//!
//! Provides safe RAII wrappers over `llama.cpp` and `ggml` backend dispatch.

pub mod error;
pub mod ffi_bridge;
pub mod backend_loader;

pub use error::EngineError;
pub use ffi_bridge::{
    ContextParams, LlamaBackendGuard, LlamaBackendManager, ModelLoadParams, SafeLlamaBatch,
    SafeLlamaContext, SafeLlamaModel, SafeSamplerChain,
};
pub use backend_loader::{DynamicBackendLoader, GgmlBackendDescriptor};
