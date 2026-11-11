//! Universal Mobile Compute Fabric (UMCF) GPU Compute & Distributed Rendering Engine.
//!
//! Provides:
//! - Sandboxed WGSL/SPIR-V compute shader validation with AST loop-fuel watchdog injection.
//! - Bounded VRAM memory allocation with strict zero-leak page scrubbing.
//! - Time-sliced compute kernel dispatching with asynchronous DMA readbacks.

#![forbid(unsafe_code)]

pub mod compositor;
pub mod dispatcher;
pub mod error;
pub mod render_coordinator;
pub mod shader_sandbox;
pub mod verifier;

pub use compositor::{CompletedTilePacket, FrameCompositor, ProgressiveFrameDelta};
pub use dispatcher::{
    BoundedVramAllocator, BufferUsage, ComputeExecutionReceipt, DispatchWorkgroups, GpuBuffer,
    GpuComputeDispatcher, KernelBindings,
};
pub use error::GpuError;
pub use render_coordinator::{
    morton_2d, DistributedRenderCoordinator, RenderJobSpec, RenderPixelFormat, TileDescriptor,
    TileState, WorkerThroughputTracker,
};
pub use shader_sandbox::{
    BoundsCheckPolicy, ShaderResourceLimits, ShaderSandbox, StorageBufferBinding,
    UniformBufferBinding, ValidatedShader,
};
pub use verifier::{
    compute_max_abs_diff, compute_psnr, compute_ssim, extract_subtile_crop, has_nan_or_inf_pixels,
    select_canary_crop, RenderOutputVerifier, SubtileCropBox, TileVerificationReceipt,
    VerificationFailureEvidence, VerificationPolicy,
};
