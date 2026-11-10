# Subphase S5.P2.SP2 Execution Prompt: Hardware Backend Selection (CPU AVX/NEON, Vulkan, Metal, CUDA/ROCm) with Fallback

## 1. Role & Context
You are the Heterogeneous Compute & GPU Acceleration Engineer for UMCF. In Subphase `S5.P2.SP2`, you will implement `HardwareBackendSelector` and the resilient fallback state machine in `crates/umcf-ai/src/backend_selector.rs`. Mobile and desktop devices exhibit wide diversity in GPU drivers, unified vs discrete VRAM, SIMD extensions, and thermal envelopes; your selector must automatically pick the highest-throughput backend configuration (`n_gpu_layers`, `split_mode`, `n_threads`, `flash_attn`) and gracefully degrade without crashing if a GPU driver fails or thermal throttling strikes.

## 2. Prerequisites & Dependencies
- `S5.P2.SP1` (`SafeLlamaModel`, `SafeLlamaContext`, `LlamaBackendManager`).
- `S5.P1.SP2` (`ModelFootprintEstimator`).
- `crates/umcf-hw` (`HardwareProfiler`, live VRAM/RAM telemetry, thermal state notifications).

## 3. Implementation Tasks
1. **Backend Discovery & Synthetic Micro-Probe (`crates/umcf-ai/src/backend_selector.rs`)**:
   - Enumerate all GGML backend devices (`ggml_backend_dev_get`, `ggml_backend_dev_type`, `ggml_backend_dev_memory`):
     - `GpuBackend::Cuda { device_idx, compute_capability, free_vram }`
     - `GpuBackend::Rocm { device_idx, arch_name, free_vram }`
     - `GpuBackend::Metal { unified_memory_limit, supports_bfloat16 }`
     - `GpuBackend::Vulkan { device_name, subgroup_size, fp16_storage, cooperative_matrix }`
     - `CpuBackend { isa_flags: CpuIsaFlags (AVX2, AVX512_VNNI, AMX, ARM_NEON, ARM_DOTPROD, ARM_I8MM), performance_cores, efficiency_cores }`
   - Implement a 10 ms matmul/quant-dot micro-probe (`run_backend_sanity_probe`) on first startup (cached by driver version hash) to verify the GPU shader compiler produces numerically valid results (catching buggy mobile Adreno/Mali Vulkan drivers) before trusting a GPU backend.
2. **Optimal Layer Offload & Thread Tuning (`BackendExecutionPlan`)**:
   - Compute `n_gpu_layers` (`0..=total_layers`) using `ModelFootprintEstimator` and current free VRAM minus OS safety margin.
   - Set `n_threads` (generation) to the count of physical Performance (P) cores on big.LITTLE / Apple Silicon / Intel Hybrid CPUs to avoid E-core straggler synchronization penalties, and `n_threads_batch` (prompt prefill) to all physical cores.
3. **Resilient Fallback State Machine (`FallbackExecutor`)**:
   - Implement a 4-tier fallback chain when loading a model or creating a context:
     1. **Tier 1 (Full GPU Offload)**: `n_gpu_layers = all` on preferred discrete/unified GPU (`CUDA`/`ROCm`/`Metal`/`Vulkan`).
     2. **Tier 2 (Hybrid Partial GPU Offload)**: Automatically binary-search or step down `n_gpu_layers` by 25% increments if VRAM allocation returns `None`/error.
     3. **Tier 3 (CPU SIMD Accelerated)**: `n_gpu_layers = 0`, using AVX-512/AVX2/NEON/dotprod with `mmap = true`.
     4. **Tier 4 (Safe Mode CPU)**: Single-socket P-core threads, `flash_attn = false`, reduced `n_ubatch = 128`.
   - Subscribe to `umcf-hw` `ThermalEvent::Severe` / `BatteryLow`: dynamically migrate or reload active worker context to lower GPU utilization or reduce batch size between requests.
4. **UI Integration**:
   - Flutter & Svelte 5: Backend selector dropdown (`Auto (Recommended)`, `Vulkan`, `Metal`, `CUDA`, `CPU Only`) with live active-tier indicator, GPU layer offload slider, and fallback history warning banner.

## 4. Acceptance Criteria
- >= 95% line/branch coverage in `crates/umcf-ai/src/backend_selector.rs`; 100% test pass rate.
- If a Vulkan/CUDA/Metal backend fails the numerical sanity probe (NaN or > 1e-2 L_inf error vs CPU reference on a $64 \times 64$ Q4_K matmul) or returns an OOM error during load, `FallbackExecutor` automatically transitions to the next tier within <= 500 ms with zero panics or process crashes.
- On hybrid CPU architectures (Intel P+E cores, ARM big.LITTLE), default decode thread selection binds exclusively to performance cores.

## 5. Required Tests
- **`[UNIT]`**:
  - `test_gpu_layer_offload_calculation_with_vram_headroom`
  - `test_big_little_p_core_thread_selection_logic`
  - `test_fallback_state_machine_tier1_to_tier4_transitions`
  - `test_numerical_sanity_probe_rejects_corrupted_gpu_output`
- **`[INT]`**:
  - Inject synthetic OOM on Tier 1 and Tier 2 during `SafeLlamaModel::load_verified`, verify automatic recovery on Tier 3 (CPU SIMD) and successful token generation.
- **`[SIM]`**:
  - Simulate `ThermalEvent::Critical` on an active mobile node and verify dynamic batch-size & GPU layer step-down.
- **`[HW]`**:
  - Execute and record tokens/sec across CPU SIMD and available GPU backend (Vulkan/Metal/CUDA) on real hardware.
- **`[MANUAL]`**:
  - Toggle backend override in Flutter/Svelte UI between GPU and CPU, verifying telemetry badge and VRAM usage graph update in real time.

## 6. Required Evidence & Artifacts
- `artifacts/stage-05/p2/sp2_backend_probe_and_fallback.json`
- `artifacts/stage-05/p2/sp2_hardware_throughput_comparison.json`

## 7. Documentation, `TODO.md` & `CHANGELOG.md` Updates
- Document the 4-tier backend fallback state machine and big.LITTLE thread pinning rules in `docs/architecture/AI_INFERENCE_ENGINE.md` (Section 2).
- Mark `S5.P2.SP2` complete in `TODO.md` and update `CHANGELOG.md`.

## 8. Rollback Criteria & Completion Gate
- **Rollback**: If a broken mobile Vulkan driver can crash the UMCF daemon process instead of being caught by the sanity probe/fallback, or if CPU thread selection oversubscribes efficiency cores and degrades decode speed.
- **Completion Gate**: All tests green with >= 95% coverage before proceeding to `S5.P2.SP3`.
