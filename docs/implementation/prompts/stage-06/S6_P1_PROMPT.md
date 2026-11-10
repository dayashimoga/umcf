# Phase S6.P1 Execution Prompt: Cross-Platform GPU Compute & Distributed Rendering Engine

## 1. Role & Context
You are the GPU Compute & Distributed Graphics Lead for UMCF. Phase `S6.P1` builds `crates/umcf-gpu`, providing sandboxed cross-platform GPU compute shader execution (`wgpu` over Vulkan, Metal, DX12, and WebGPU), work-stealing tile/frame-partitioned distributed rendering across mobile and desktop GPUs, and deterministic/perceptual verification of rendered outputs against Byzantine or faulty GPU drivers.

This phase consists of three sequential subphases:
1. `S6_P1_SP1.md`: WGPU/Vulkan/Metal Compute Shader Dispatch & Buffer Validation
2. `S6_P1_SP2.md`: Tile/Frame-Partitioned Distributed Rendering Coordinator & Compositor
3. `S6_P1_SP3.md`: Deterministic Verification & Checksum Auditing of Rendered Outputs

## 2. Prerequisites & Dependencies
- Stage 1–5 completion (`umcf-scheduler`, `umcf-net`, `umcf-hw`).
- `wgpu` >= 22.0 and `naga` with WGSL and SPIR-V frontends/backends enabled.

## 3. Implementation Tasks
1. **Subphase S6.P1.SP1 (`crates/umcf-gpu/src/shader_sandbox.rs`, `src/dispatcher.rs`)**:
   - Parse and validate incoming WGSL/SPIR-V compute shaders via `naga`, enforcing buffer bounds checks (`BoundsCheckPolicy::ReadZeroSkipWrite`), workgroup size limits, binding layout caps, and loop-iteration watchdog instrumentation to prevent GPU Timeout Detection and Recovery (TDR) hangs.
   - Implement `GpuComputeDispatcher` managing pooled staging/storage buffers, async DMA readbacks, and hardware timestamp queries.
2. **Subphase S6.P1.SP2 (`crates/umcf-gpu/src/render_coordinator.rs`, `src/compositor.rs`)**:
   - Implement `DistributedRenderCoordinator` decomposing 2D frames (into Morton-curve / Hilbert-curve spatial tiles) and 3D animations (into frame ranges + tile sub-regions) with dynamic work-stealing so fast desktop GPUs consume tiles at 10x the rate of mobile GPUs without straggler bottlenecks.
   - Implement `FrameCompositor` assembling compressed EXR/PNG/WebP/RAW tiles into final output frames with progressive live preview streaming.
3. **Subphase S6.P1.SP3 (`crates/umcf-gpu/src/verifier.rs`)**:
   - Implement exact BLAKE3 checksum auditing for integer/deterministic compute kernels and canary spot-check rendering + perceptual SSIM/PSNR/L_inf tolerance verification for floating-point raytracing/path-tracing shaders across heterogeneous GPU architectures.

## 4. Acceptance Criteria
- >= 95% line/branch coverage across `crates/umcf-gpu`; 100% test pass rate.
- Malicious WGSL shaders containing out-of-bounds storage buffer writes or infinite `loop {}` constructs are safely bounded/terminated within <= 250 ms without crashing the GPU device or host process.
- Dynamic tile work-stealing keeps final frame completion idle-tail time (difference between last node finishing and first node finishing) under <= 5% of total frame render time across a 10:1 speed-ratio heterogeneous cluster.
- Canary spot-checking detects 100% of corrupted or black/NaN tiles injected by a faulty or malicious worker node and automatically re-schedules them on a healthy peer.

## 5. Required Tests
- **`[UNIT]`**: Naga AST loop-bound transform, Morton-order tile generator, work-stealing queue prioritization, SSIM/PSNR/NaN detector math.
- **`[INT]`**: Multi-worker distributed raytracing / Mandelbrot / matrix-compute job end-to-end execution and progressive tile composition.
- **`[SIM]`**: Inject NaN/black pixels and 500x straggler latency on 1 of 4 render workers; verify spot-check rejection and work-stealing recovery.
- **`[HW]`**: Execute identical WGSL compute shader across Vulkan, Metal, and DX12/CPU adapters and verify cross-vendor output tolerance.
- **`[MANUAL]`**: Watch live progressive tile fill-in on Flutter `DistributedRenderStudioScreen` and Svelte 5 `/gpu` canvas.

## 6. Required Evidence & Artifacts
- `artifacts/stage-06/p1/shader_sandbox_security_report.json`
- `artifacts/stage-06/p1/work_stealing_straggler_timeline.json`
- `artifacts/stage-06/p1/cross_gpu_determinism_audit.json`

## 7. Documentation, `TODO.md` & `CHANGELOG.md` Updates
- Document WGSL sandbox rules, tile work-stealing protocol, and cross-GPU verification tolerances in `docs/architecture/GPU_RENDER_AND_MEDIA.md` (Part 1).
- Mark `S6.P1.SP1`–`S6.P1.SP3` complete in `TODO.md` and update `CHANGELOG.md`.

## 8. Rollback Criteria & Completion Gate
- **Rollback**: If any shader can read uninitialized VRAM from a previous tenant's buffer or hang the OS window server via GPU TDR.
- **Gate**: All 3 subphases (`S6_P1_SP1`..`SP3`) pass with >= 95% coverage before starting Phase `S6.P2`.
