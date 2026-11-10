# Subphase S6.P1.SP1 Execution Prompt: WGPU/Vulkan/Metal Compute Shader Dispatch & Buffer Validation

## 1. Role & Context
You are the GPU Systems & Shader Sandbox Engineer for UMCF. In Subphase `S6.P1.SP1`, you will implement `crates/umcf-gpu/src/shader_sandbox.rs` and `src/dispatcher.rs`, providing safe, zero-leak, bounds-checked execution of WGSL and SPIR-V compute workloads across Vulkan, Metal, DirectX 12, and WebGPU backends. Because remote cluster nodes execute untrusted or third-party compute shaders, every shader and buffer binding must be statically validated and instrumented against out-of-bounds memory access, uninitialized VRAM leakage, and infinite-loop GPU TDR hangs.

## 2. Prerequisites & Dependencies
- `crates/umcf-hw` (GPU adapter enumeration, VRAM limits, thermal telemetry).
- `wgpu` and `naga` crates configured in workspace `Cargo.toml`.

## 3. Implementation Tasks
1. **Static WGSL/SPIR-V Validation & Loop-Bound Instrumentation (`crates/umcf-gpu/src/shader_sandbox.rs`)**:
   - Parse incoming WGSL or SPIR-V modules into `naga::Module` and validate with `naga::valid::Validator` enforcing `ValidationFlags::all()` and `Capabilities::default()`.
   - Enforce `ShaderResourceLimits`:
     - Max workgroup size ($X \times Y \times Z \le 1024$), max storage buffers per stage ($\le 8$), max uniform buffer size ($\le 64\text{ KiB}$), max shared workgroup memory ($\le 32\text{ KiB}$).
   - **AST Loop Watchdog Injection (`inject_loop_fuel_watchdog`)**:
     - Traverse every `Statement::Loop` in the `naga::Module` AST and inject a thread-local fuel counter (`var _umcf_loop_fuel: u32 = 100000u;`) that decrements on each iteration and issues a `break` (and sets an atomic abort flag in a hidden diagnostic binding) if exhausted, preventing infinite-loop GPU TDR resets on desktop/mobile drivers.
   - Configure `wgpu::ShaderRuntimeChecks { bounds_checks: true, force_loop_bounding: true }` and `naga::back::BoundsCheckPolicies` (`ReadZeroSkipWrite` for storage, uniform, and image accesses).
2. **VRAM Buffer Pool & Zero-Leak Compute Dispatcher (`crates/umcf-gpu/src/dispatcher.rs`)**:
   - Implement `BoundedVramAllocator` tracking active buffer allocations against a configurable per-node VRAM ceiling (e.g., 50% of free VRAM on mobile, 80% on desktop).
   - Zero-initialize all newly allocated or recycled GPU storage buffers (`clear_buffer`) before binding so a compute task can **never** read residual VRAM contents from a prior workload.
   - Implement `GpuComputeDispatcher::dispatch_kernel(&ValidatedShader, &KernelBindings, DispatchWorkgroups) -> Result<ComputeExecutionReceipt, GpuError>`:
     - chunks large 3D dispatches into time-sliced sub-dispatches ($\le 50\text{ ms}$ per command encoder submission) so mobile UI compositors never drop frames and thermal/cancellation interrupts can preempt between slices.
     - Collects hardware GPU timestamp queries (`wgpu::Features::TIMESTAMP_QUERY` when supported, with CPU wall-clock fallback) and returns output buffers via async `map_async` staging readback.

## 4. Acceptance Criteria
- >= 95% line/branch coverage in `shader_sandbox.rs` and `dispatcher.rs`; 100% test pass rate.
- Submitting a WGSL shader containing `loop { storage_buf[0] += 1u; }` (infinite loop) terminates cleanly within <= 150 ms via the injected loop fuel watchdog and returns `GpuError::ShaderLoopFuelExhausted` without triggering an OS GPU driver reset.
- Submitting a shader that attempts to read/write `storage_buf[99999999]` on a 1024-byte buffer safely clamps/skips the access via `ReadZeroSkipWrite` without segfaulting or corrupting adjacent allocations.
- Zero residual data leakage: allocating a buffer after freeing a buffer filled with `0xDEADBEEF` reads back all `0x00000000`.

## 5. Required Tests
- **`[UNIT]`**:
  - `test_naga_ast_loop_fuel_watchdog_injection_nested_loops`
  - `test_shader_resource_limits_reject_excessive_workgroup_memory`
  - `test_vram_allocator_quota_enforcement_and_zero_scrub_on_recycle`
- **`[INT]`**:
  - `test_vector_add_and_matrix_multiply_wgsl_dispatch_and_readback`
  - `test_infinite_loop_shader_aborts_gracefully_with_diagnostic_flag`
  - `test_out_of_bounds_buffer_read_returns_zero_and_write_is_skipped`
- **`[SIM]`**:
  - 100 concurrent compute dispatch requests with randomized buffer sizes and simulated cancellations between time-sliced sub-dispatches.
- **`[HW]`**:
  - Execute compute benchmark suite on native Vulkan / Metal / DX12 adapter and record GFLOPS and dispatch overhead.
- **`[MANUAL]`**:
  - Open Svelte 5 Shader Sandbox (`/gpu`), paste a test WGSL raymarching shader and an intentional infinite-loop shader, and verify live execution vs safe watchdog termination banner.

## 6. Required Evidence & Artifacts
- `artifacts/stage-06/p1/sp1_shader_sandbox_audit.json`
- `artifacts/stage-06/p1/sp1_gpu_dispatch_benchmarks.json`

## 7. Documentation, `TODO.md` & `CHANGELOG.md` Updates
- Document WGSL AST loop-fuel injection and VRAM buffer zero-scrubbing in `docs/architecture/GPU_RENDER_AND_MEDIA.md` (Section 1.1).
- Mark `S6.P1.SP1` complete in `TODO.md` and update `CHANGELOG.md`.

## 8. Rollback Criteria & Completion Gate
- **Rollback**: If any untrusted WGSL/SPIR-V payload can bypass `naga` validation, read unzeroed VRAM, or hang the GPU command queue for > 500 ms.
- **Completion Gate**: All tests green with >= 95% coverage before proceeding to `S6.P1.SP2`.
