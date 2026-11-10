# Subphase S8.P1.SP3 Execution Prompt: Long-Haul Soak Testing & Zero-Leak Memory/FD Profiling

## 1. Role & Context
You are the Systems Performance & Resource Leak Profiling Engineer for UMCF. In Subphase `S8.P1.SP3`, you will implement `tests/soak/src/soak_runner.rs`, a high-throughput accelerated soak testing and resource profiling suite. Because the UMCF daemon runs continuously for weeks or months as a background service on mobile phones and desktop computers, even a tiny leak of 4 KiB of RAM, 1 GPU buffer handle, 1 SQLite statement, or 1 UDP socket per job would eventually crash the user's device.

## 2. Prerequisites & Dependencies
- `S8.P1.SP1` & `S8.P1.SP2` (`MultiOsClusterHarness`, `ChaosNemesis`).
- Profiling instrumentation (`jemalloc`/`mimalloc` stats or custom tracking `GlobalAlloc` wrapper in test builds, OS `/proc/self/fd` / `GetProcessHandleCount` / `mach_task_basic_info` inspectors, and `wgpu`/GGML buffer counters).

## 3. Implementation Tasks
1. **High-Velocity Continuous Soak Harness (`tests/soak/src/soak_runner.rs`)**:
   - Implement `SoakTestCoordinator` executing continuous waves of heterogeneous workloads (configurable duration: 10,000 accelerated job cycles in CI; 24–72 hour wall-clock mode for hardware certification):
     - Wave mix per cycle:
       1. 5 WASM sandboxed compute jobs (loaded, executed, unloaded).
       2. 2 LLM inference sessions (context created, 64 tokens streamed, 1 cancelled mid-stream, KV slot freed).
       3. 2 GPU compute/render dispatches (buffers allocated, shader run, staging buffers unmapped & dropped).
       4. 1 GOP-segmented video transcode + stitch + temp file cleanup.
       5. 1 Remote streaming session connect, 120 frames + 500 input events, abrupt disconnect (testing stuck-key release & encoder teardown).
       6. 5 Storage chunk ingestions + pins + unpins + periodic `ConcurrentGarbageCollector::run_gc_cycle()`.
       7. Periodic background Chaos Nemesis pulses (peer reconnects, QUIC 0-RTT resumptions).
2. **Automated Zero-Leak Regression Profiler (`tests/soak/src/leak_profiler.rs`)**:
   - Sample system and internal resource metrics after every 100 job cycles (following a forced idle + GC quiescence barrier):
     - `heap_allocated_bytes` & `rss_bytes`
     - `vram_allocated_bytes` & `wgpu_active_buffer_count`
     - `llama_active_contexts` & `llama_active_models`
     - `open_file_descriptors` / Windows `handle_count`
     - `active_quic_connections` & `active_quic_streams`
     - `active_tokio_tasks` & `temp_directory_file_count`
   - Compute Ordinary Least Squares (OLS) linear regression slope ($\text{bytes\_per\_cycle}$ and $\text{handles\_per\_cycle}$) over cycles `[2_000 ..= 10_000]` (post-allocator-warmup):
     - **Hard Assertion**: Slope of `heap_allocated_bytes`, `vram_allocated_bytes`, `open_file_descriptors`, `active_quic_streams`, and `temp_directory_file_count` must be $\le 0.0$ (within statistical noise $\epsilon < 10^{-3}$), and post-quiescence gauge counters (`wgpu_active_buffer_count`, `llama_active_contexts`, `temp_directory_file_count`) must return to **exact `0`**.

## 4. Acceptance Criteria
- >= 95% line/branch coverage of the soak & leak profiling suite; 100% pass rate.
- After 10,000 full workload lifecycles + final quiescence barrier:
  - `leaked_temp_files == 0`
  - `leaked_vram_buffers == 0`
  - `leaked_llama_contexts == 0`
  - `open_file_descriptors == baseline_idle_fds` ($\pm 0$)
  - `heap_allocated_bytes` is within $\le 1.0\%$ of the post-warmup cycle-1,000 baseline.
- Valgrind / AddressSanitizer + LeakSanitizer (LSan) reports **0 definitely lost, 0 indirectly lost, and 0 possibly lost bytes** on clean daemon shutdown after a soak run.

## 5. Required Tests
- **`[UNIT]`**:
  - `test_ols_linear_regression_leak_slope_detector_catches_16_byte_per_cycle_leak`
  - `test_fd_and_os_handle_counter_accuracy_windows_linux_macos`
  - `test_quiescence_barrier_waits_for_background_drop_tasks`
- **`[INT]`**:
  - `test_1000_cycle_ci_accelerated_multi_workload_soak_zero_leak_gate`
  - `test_abrupt_client_disconnect_storm_zero_socket_or_stream_handle_leak`
- **`[SIM]`**:
  - 10,000-cycle accelerated soak test with periodic peer churn and mid-job cancellations.
- **`[HW]`**:
  - Multi-hour continuous hardware soak test monitoring physical RAM, GPU VRAM (`nvidia-smi` / Metal / Vulkan memory heaps), and CPU temperature stability.
- **`[MANUAL]`**:
  - Inspect the "System Resource & Leak Diagnostics" chart in Svelte 5 `/settings` during a soak run, verifying flat horizontal memory/handle trendlines.

## 6. Required Evidence & Artifacts
- `artifacts/stage-08/p1/sp3_10000_cycle_soak_regression_report.json`
- `artifacts/stage-08/p1/sp3_lsan_shutdown_zero_leak_proof.txt`

## 7. Documentation, `TODO.md` & `CHANGELOG.md` Updates
- Document resource lifecycle invariants and soak profiling methodology in `docs/OPERATIONS_RUNBOOK.md` (Section 3).
- Mark `S8.P1.SP3` and Phase `S8.P1` complete in `TODO.md`; update `CHANGELOG.md`.

## 8. Rollback Criteria & Completion Gate
- **Rollback**: Any positive memory, VRAM, file descriptor, or temporary file leak slope detected over the soak run.
- **Completion Gate**: Phase `S8.P1` completion gate passed (`S8_P1_SP1`..`SP3` green). Proceed to Phase `S8.P2`.
