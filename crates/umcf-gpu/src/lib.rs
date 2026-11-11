//! Universal Mobile Compute Fabric (UMCF) GPU Compute & Distributed Rendering Engine.
//!
//! Provides:
//! - Sandboxed WGSL/SPIR-V compute shader validation with AST loop-fuel watchdog injection.
//! - Bounded VRAM memory allocation with strict zero-leak page scrubbing.
//! - Time-sliced compute kernel dispatching with asynchronous DMA readbacks.

#![forbid(unsafe_code)]

pub mod dispatcher;
pub mod error;
pub mod shader_sandbox;

pub use dispatcher::{
    BoundedVramAllocator, BufferUsage, ComputeExecutionReceipt, DispatchWorkgroups,
    GpuBuffer, GpuComputeDispatcher, KernelBindings,
};
pub use error::GpuError;
pub use shader_sandbox::{
    BoundsCheckPolicy, ShaderResourceLimits, ShaderSandbox, StorageBufferBinding,
    UniformBufferBinding, ValidatedShader,
};
