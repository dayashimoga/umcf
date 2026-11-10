# Universal Mobile Compute Fabric (UMCF) — AI Inference Engine Specification

## 1. Native `llama.cpp` Engine FFI Architecture & Lifetime Invariants

The UMCF AI Inference Engine is built on top of `llama.cpp` and `ggml` through strict, panic-free Rust FFI bindings divided into two crates:
- `crates/umcf-llama-sys`: Low-level, audited C ABI bindings and C engine wrapper adhering to Rule 01 FFI exception list.
- `crates/umcf-ai`: Safe RAII abstractions, ownership wrappers, backend selection, and streaming token generation.

```
+-------------------------------------------------------------------------+
|                              umcf-ai                                    |
|  +--------------------+        holds Arc         +-------------------+  |
|  |  SafeLlamaContext  | -----------------------> |  SafeLlamaModel   |  |
|  |  (!Sync, Send)     |                          |  (Send + Sync)    |  |
|  +--------------------+                          +-------------------+  |
|            |                                               |            |
|            | calls                                         | calls      |
|            v                                               v            |
+-------------------------------------------------------------------------+
|                            umcf-llama-sys                               |
|  +--------------------+                          +-------------------+  |
|  | llama_decode()     |                          | llama_load_...()  |  |
|  | llama_free()       |                          | llama_model_free()|  |
|  +--------------------+                          +-------------------+  |
|            |                                               |            |
|            v                                               v            |
|  +-------------------------------------------------------------------+  |
|  |          Native C/C++ Engine Runtime (`llama.h`, `ggml`)          |  |
|  +-------------------------------------------------------------------+  |
+-------------------------------------------------------------------------+
```

### 1.1 Structural Lifetime & Ownership Invariants

1. **Certified Loading via `VerifiedModelGuard`**:
   - `SafeLlamaModel` cannot be constructed from raw file paths or untrusted pointers.
   - It is instantiated *exclusively* via:
     ```rust
     SafeLlamaModel::load_verified(guard: &VerifiedModelGuard, params: &ModelLoadParams) -> Result<Arc<Self>, EngineError>
     ```
   - This enforces at compile time that only cryptographically certified (BLAKE3 leaf + Merkle root verified) GGUF files reach the native engine.

2. **Model Outlives Context Guarantee**:
   - `SafeLlamaContext` stores an `Arc<SafeLlamaModel>`.
   - Even if the caller drops their own `SafeLlamaModel` handle, the underlying C++ model weights are guaranteed to remain resident in memory until all child contexts are destroyed.

3. **Thread Safety & Aliasing Discipline**:
   - `SafeLlamaModel` is `Send + Sync`: model weights are read-only and immutable after load.
   - `SafeLlamaContext` is `Send` but `!Sync`: decoding, KV-cache mutation, and logit updates require exclusive `&mut self` access.

4. **Cooperative Cancellation**:
   - Model loading supports cooperative cancellation via `ModelLoadParams.progress_callback`. If the callback returns `false`, loading is immediately aborted and all intermediate allocations freed in `< 20 ms`.
   - Decoding supports instantaneous abort via `SafeLlamaContext.abort()`, backed by `ggml_abort_callback`, halting in-flight execution within `< 50 ms`.

5. **Structured Logging Redirection**:
   - Native `llama.cpp` logs are captured via `llama_log_set` and routed into Rust `tracing` with target `"umcf_ai::llama_cpp"`. Zero string allocation occurs when the target log level is inactive.

### 1.2 Dynamic GGML Backend Plugin Loader

Dynamic backend plugins (`.dll`, `.so`, `.dylib`) are verified prior to loading:
- The SHA-256 / BLAKE3 digest of the plugin file is calculated and compared against the authoritative release manifest.
- Signature mismatches are rejected immediately with `EngineError::BackendPluginSignatureMismatch`.
- Valid plugins are registered into the backend registry via `ggml_backend_load_all_from_path`.
