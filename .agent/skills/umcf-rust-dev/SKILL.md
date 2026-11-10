---
name: umcf-rust-dev
description: >-
  Guides Rust development across the UMCF workspace, including async Tokio architecture, llama.cpp FFI bindings, hardware probing, memory pressure governance, and zero-warning Clippy compliance. Use when writing, refactoring, or debugging Rust crates in UMCF.
---

# UMCF Rust Engineering & `llama.cpp` Integration Skill

Use this skill when developing or modifying any Rust crate in `crates/*`.

---

## 1. Crate Safety & Lint Configuration

1. **Workspace Lints (`Cargo.toml`)**:
   Ensure every pure-Rust crate inherits workspace lints and enforces `#![forbid(unsafe_code)]`:
   ```rust
   #![forbid(unsafe_code)]
   #![deny(
       clippy::all,
       clippy::pedantic,
       clippy::unwrap_used,
       clippy::expect_used,
       clippy::panic,
       clippy::todo,
       clippy::unimplemented,
       clippy::dbg_macro
   )]
   ```
2. **Audited FFI Crates (`umcf-llama-sys`, `umcf-hw-sys`)**:
   - Isolate all `unsafe` C/C++ bindings inside `*-sys` crates.
   - Wrap every C pointer (`llama_model*`, `llama_context*`, `ggml_backend_t`) in a safe RAII struct implementing `Drop` with deterministic cleanup.
   - Guard every C callback or FFI entry point with `std::panic::catch_unwind` so Rust panics never unwind across C/Dart ABI boundaries.

---

## 2. Structured Error Handling Pattern

Define domain-specific error types using `thiserror` with machine-readable error codes:

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum InferenceError {
    #[error("insufficient available memory: required {required_mb} MB, available {available_mb} MB")]
    InsufficientMemory { required_mb: u64, available_mb: u64 },

    #[error("model shard integrity check failed: expected BLAKE3 {expected}, got {actual}")]
    ShardIntegrityMismatch { expected: String, actual: String },

    #[error("hardware backend '{backend}' is unavailable: {reason}")]
    BackendUnavailable { backend: String, reason: String },

    #[error("pipeline stage {stage_index} timed out after {timeout_ms} ms on peer {peer_id}")]
    PipelineStageTimeout {
        stage_index: u32,
        peer_id: String,
        timeout_ms: u64,
    },
}
```

---

## 3. Distributed `llama.cpp` Execution Architecture

When implementing local or distributed LLM inference in `umcf-llama-engine`:

1. **Pre-Flight GGUF Header Inspection**:
   - Parse GGUF metadata headers before loading tensor weights into memory.
   - Compute exact memory requirements:
     $$\text{RAM}_{\text{stage}} = \text{Bytes}_{\text{assigned\_layers}} + \text{Bytes}_{\text{KV\_cache}}(\text{n\_ctx}, \text{n\_layer\_stage}) + \text{Bytes}_{\text{compute\_buffer}}$$
   - Query `ResourceGovernor::request_allocation(ram_stage)` prior to calling `llama_load_model_from_file`.
2. **Pipeline Layer Partitioning**:
   - Split model layers `[0..N)` across ordered pipeline stages according to each participating node's usable memory and compute throughput.
   - **Head Node (Stage 0)**: Tokenizes prompt, runs embedding + layers `[0..L_1)`, serializes hidden state tensor (shape `[n_tokens, n_embd]`, FP16 or quantized activation framing), and streams it over a dedicated QUIC stream to Stage 1.
   - **Intermediate Nodes (Stage $k$)**: Deserializes incoming activation tensor with strict shape/bounds validation, runs layers `[L_k..L_{k+1})`, updates local KV cache for its assigned layers, and forwards activations to Stage $k+1$.
   - **Tail Node (Stage $M-1$)**: Runs final layers + normalization + output logits projection, samples next token (or returns top-k logits), and streams token ID back to the Head Node.
3. **Zero Blocking on Tokio Executor**:
   - Always run `llama_decode` / `llama_encode` on a dedicated pinned thread pool or `tokio::task::spawn_blocking` with cooperative cancellation between token steps.

---

## 4. Concurrency, Backpressure & Resource Cleanup

1. **Bounded Channels**:
   - Use `tokio::sync::mpsc::channel(capacity)` everywhere.
   - For real-time telemetry or video frames, use `tokio::sync::watch` or bounded ring buffers that drop stale frames rather than accumulating latency.
2. **Deterministic Resource Teardown**:
   - Implement `shutdown(&self, token: CancellationToken)` on every service.
   - Verify in tests that dropping or shutting down a node frees all UDP sockets, file locks, and model buffers within 500 ms.
