# Subphase S5.P2.SP1 Execution Prompt: Safe Rust FFI Bindings to `llama.cpp` & GGML Backend Loader

## 1. Role & Context
You are the Systems FFI & Native Toolchain Engineer for UMCF. In Subphase `S5.P2.SP1`, you will build `crates/umcf-llama-sys` (low-level `bindgen` + CMake bindings to `llama.cpp` and `ggml`) and `crates/umcf-ai/src/ffi_bridge.rs` (100% safe, lifetime-enforced Rust wrappers). Every C pointer, callback, and backend handle must be encapsulated in RAII types that enforce Rust aliasing, thread-safety (`Send`/`Sync`), and deterministic teardown.

## 2. Prerequisites & Dependencies
- `S5.P1.SP3` (`VerifiedModelGuard`).
- Pinned `llama.cpp` C/C++ source tree in `vendor/llama.cpp` with SHA-256 lockfile verification.

## 3. Implementation Tasks
1. **Cross-Platform `build.rs` & Bindgen (`crates/umcf-llama-sys/build.rs`)**:
   - Configure CMake build of `ggml`, `ggml-base`, `ggml-cpu`, and optional dynamic backend plugins (`ggml-vulkan`, `ggml-metal`, `ggml-cuda`, `ggml-hip`, `ggml-rpc`).
   - Enforce CPU instruction set targets without illegal instruction crashes on older CPUs (compile runtime-dispatched CPU backend variants or explicit feature flags `avx2`, `avx512`, `neon`, `dotprod`).
   - Generate bindings with `bindgen::Builder` whitelisting `llama_*` and `ggml_*` types/functions.
2. **Safe RAII FFI Bridge (`crates/umcf-ai/src/ffi_bridge.rs`)**:
   - Implement singleton `LlamaBackendManager` guarding `llama_backend_init()` and `llama_backend_free()` via `OnceLock` + reference counting, plus structured log redirection (`llama_log_set`) routed into `tracing` without string allocation on disabled log levels.
   - Implement `SafeLlamaModel`:
     - Constructed *exclusively* via `SafeLlamaModel::load_verified(guard: &VerifiedModelGuard, params: &ModelLoadParams) -> Result<Arc<Self>, EngineError>`.
     - Wires `progress_callback` to a Rust closure returning `bool` so model loading can be cancelled mid-mmap.
     - Implements `Drop` calling `llama_model_free(self.ptr.as_ptr())`.
     - Marked `Send + Sync` (since `llama_model` weights are immutable after load).
   - Implement `SafeLlamaContext`:
     - Holds `Arc<SafeLlamaModel>` so the underlying `llama_model` can **never** be dropped while any `llama_context` references it.
     - Marked `Send` but `!Sync` (enforcing exclusive `&mut self` access for `decode`/`encode` calls).
     - Implements `Drop` calling `llama_free(self.ctx_ptr.as_ptr())`.
   - Implement `SafeLlamaBatch` and `SafeSamplerChain` with bounds-checked token/pos/seq_id population and automatic `llama_batch_free` / `llama_sampler_free`.
3. **Dynamic GGML Backend Plugin Loader (`crates/umcf-ai/src/backend_loader.rs`)**:
   - Enumerate compiled-in and dynamically loadable GGML backend shared libraries (`ggml_backend_load_all_from_path`) after verifying the shared library's SHA-256 signature against the UMCF binary manifest.

## 4. Acceptance Criteria
- >= 95% line/branch coverage in `crates/umcf-ai/src/ffi_bridge.rs` and `src/backend_loader.rs`; 100% test pass rate.
- Compile-time lifetime/ownership guarantees: `SafeLlamaContext` holds an `Arc<SafeLlamaModel>`, preventing use-after-free even if the caller drops its own model handle early.
- Zero leaks reported by Valgrind / ASan / LSan when loading, running 100 forward passes, and dropping `SafeLlamaModel`, `SafeLlamaContext`, `SafeLlamaBatch`, and `SafeSamplerChain`.
- Cancelling `SafeLlamaModel::load_verified` at 50% progress via the progress callback cleanly aborts C++ loading and frees all partial buffers in < 20 ms.

## 5. Required Tests
- **`[UNIT]`**:
  - `test_backend_init_idempotent_and_tracing_log_bridge`
  - `test_model_outlives_dropped_caller_handle_while_context_alive`
  - `test_batch_bounds_checking_and_raii_drop`
  - `test_model_load_abort_via_progress_callback`
- **`[INT]`**:
  - Load deterministic Tiny-GGUF fixture via `VerifiedModelGuard`, tokenize prompt (`llama_tokenize`), run `SafeLlamaContext::decode`, sample 16 greedy tokens, detokenize (`llama_token_to_piece`), and assert exact expected text output.
- **`[SIM]`**:
  - Stress test spawning 16 OS threads concurrently sharing one `Arc<SafeLlamaModel>` with 16 independent `SafeLlamaContext` instances.
- **`[HW]`**:
  - Verify `umcf-llama-sys` builds and passes FFI smoke tests on target host OS/CPU/GPU.
- **`[MANUAL]`**:
  - Inspect `tracing` logs in the UMCF daemon console confirming `llama.cpp` native logs are structured with `target = "umcf_ai::llama_cpp"`.

## 6. Required Evidence & Artifacts
- `artifacts/stage-05/p2/sp1_ffi_asan_leak_check.log`
- `artifacts/stage-05/p2/sp1_multithread_context_stress.json`

## 7. Documentation, `TODO.md` & `CHANGELOG.md` Updates
- Document `SafeLlamaModel` / `SafeLlamaContext` ownership invariants in `docs/architecture/AI_INFERENCE_ENGINE.md` (Section 1).
- Mark `S5.P2.SP1` complete in `TODO.md` and update `CHANGELOG.md`.

## 8. Rollback Criteria & Completion Gate
- **Rollback**: Any undefined behavior under Miri/ASan, any `unsafe` block outside `ffi_bridge.rs` and `backend_loader.rs`, or loading unsigned third-party `.dll`/`.so`/`.dylib` plugins.
- **Completion Gate**: All `[UNIT]`, `[INT]`, and `[SIM]` tests green with >= 95% coverage before `S5.P2.SP2`.
