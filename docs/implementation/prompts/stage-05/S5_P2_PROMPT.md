# Phase S5.P2 Execution Prompt: Native `llama.cpp` Engine Integration & Heterogeneous Backend Dispatch

## 1. Role & Context
You are the Native AI Engine & Systems Lead for UMCF. Phase `S5.P2` integrates `llama.cpp` and `ggml` via safe, panic-free, RAII-managed Rust bindings (`crates/umcf-llama-sys` and `crates/umcf-ai`), implements dynamic hardware backend discovery and fallback (CPU AVX2/AVX-512/AMX, ARM NEON/dotprod/i8mm, Vulkan, Apple Metal, NVIDIA CUDA, AMD ROCm), and builds the streaming token generation engine with paged KV-cache management, prefix caching, grammar constraints, and sub-50ms cancellation.

This phase consists of three sequential subphases:
1. `S5_P2_SP1.md`: Safe Rust FFI Bindings to `llama.cpp` & GGML Backend Loader
2. `S5_P2_SP2.md`: Hardware Backend Selection (CPU AVX/NEON, Vulkan, Metal, CUDA/ROCm) with Fallback
3. `S5_P2_SP3.md`: Streaming Token Generation, KV-Cache Management & Cancellation

## 2. Prerequisites & Dependencies
- Completion of Phase `S5.P1` (`VerifiedModelGuard`, `ModelFootprintEstimator`, `ContentAddressedModelStore`).
- Pinned, reproducible vendor submodule or source archive of `llama.cpp` (with deterministic SHA-256 verification in `crates/umcf-llama-sys/build.rs`).

## 3. Implementation Tasks
1. **Subphase S5.P2.SP1 (`crates/umcf-llama-sys`, `crates/umcf-ai/src/ffi_bridge.rs`)**:
   - Build CMake/bindgen integration in `crates/umcf-llama-sys/build.rs` supporting Windows (MSVC/Clang), Linux (x86_64/aarch64), macOS/iOS (Universal/Metal), and Android (arm64-v8a Vulkan/NEON).
   - Wrap `llama_backend_init`, `llama_model`, `llama_context`, `llama_batch`, `llama_sampler`, and `ggml_backend_reg_t` in thread-safe RAII Rust types (`LlamaBackend`, `SafeLlamaModel`, `SafeLlamaContext`, `SafeLlamaBatch`, `SafeSamplerChain`) with `Drop` implementations that guarantee zero C++ memory/resource leaks.
2. **Subphase S5.P2.SP2 (`crates/umcf-ai/src/backend_selector.rs`)**:
   - Probe all available GGML backends (`ggml_backend_dev_count`, device memory, Vulkan/Metal/CUDA/CPU capabilities) and rank them using live micro-benchmarks and thermal state from `umcf-hw`.
   - Implement resilient backend fallback state machine: if GPU context allocation fails, driver resets, or thermal throttle crosses critical threshold, seamlessly fall back to reduced `n_gpu_layers` or CPU SIMD backend without crashing the daemon.
3. **Subphase S5.P2.SP3 (`crates/umcf-ai/src/engine.rs`, `src/kv_cache.rs`)**:
   - Implement async token streaming pipeline (`TokenStream`), continuous batching / multi-sequence slot manager, KV-cache prefix tree reuse, state save/restore to disk, GBNF/JSON-schema grammar sampling, and immediate cancellation via atomic abort callback (`ggml_abort_callback`).

## 4. Acceptance Criteria
- >= 95% line/branch coverage in `crates/umcf-ai` (and full FFI lifecycle test coverage in `umcf-llama-sys`); 100% test pass rate.
- Zero memory leaks or use-after-free defects verified under AddressSanitizer (ASan) / LeakSanitizer (LSan) across 500 model load/unload and context creation/destruction cycles.
- Cancellation of an active generation aborts the in-flight `llama_decode` C++ kernel within <= 50 ms via `ggml_abort_callback`.
- Injected GPU allocation failure triggers automatic fallback to hybrid GPU/CPU or pure CPU execution within <= 500 ms with a structured telemetry event.

## 5. Required Tests
- **`[UNIT]`**: RAII drop order verification, sampler chain determinism (greedy, top-k, top-p, min-p, mirostat, grammar), KV-cache prefix matching, abort callback wiring.
- **`[INT]`**: Full model load via `VerifiedModelGuard`, prompt ingestion, streaming token generation, embedding extraction, and KV-cache slot eviction.
- **`[SIM]`**: Simulated GPU OOM (`VK_ERROR_OUT_OF_DEVICE_MEMORY`) and thermal critical alert during generation verifying automatic layer fallback.
- **`[HW]`**: Native execution on x86_64 AVX2, Apple Silicon Metal, Android ARM64 NEON + Vulkan GPU, and NVIDIA CUDA/Vulkan desktop GPU.
- **`[MANUAL]`**: Interactive playground test in Flutter & Svelte 5 selecting backends, inspecting live tokens/sec, and clicking "Stop Generation" mid-stream.

## 6. Required Evidence & Artifacts
- `artifacts/stage-05/p2/asan_lsan_valgrind_report.txt`
- `artifacts/stage-05/p2/backend_fallback_trace.json`
- `artifacts/stage-05/p2/streaming_latency_percentiles.json`

## 7. Documentation, `TODO.md` & `CHANGELOG.md` Updates
- Document FFI safety invariants, backend selection matrix, and KV-cache lifecycle in `docs/architecture/AI_INFERENCE_ENGINE.md`.
- Mark `S5.P2.SP1`–`S5.P2.SP3` complete in `TODO.md` and update `CHANGELOG.md`.

## 8. Rollback Criteria & Completion Gate
- **Rollback**: Any C++ exception or signal (`SIGSEGV`, `SIGILL`, `SIGABRT`) escaping across the Rust FFI boundary, or failure to free VRAM when a model is unloaded.
- **Gate**: All 3 subphases (`S5_P2_SP1`..`SP3`) pass with >= 95% coverage before starting Phase `S5.P3`.
