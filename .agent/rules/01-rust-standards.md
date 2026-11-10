---
trigger: always_on
description: Rust workspace coding standards, safety invariants, async bounds, serialization, and clippy gates for UMCF.
---

# 01 - Rust Workspace Standards (UMCF)

All Rust crates in the UMCF workspace (`crates/*`, FFI bridges, CLI binaries, daemon services, and edge workers) must strictly adhere to the following engineering rules.

---

## 1. Memory Safety & `unsafe` Isolation

1. **Workspace-Wide `forbid(unsafe_code)`**:
   - Every pure-Rust crate in the workspace must declare `#![forbid(unsafe_code)]` in `src/lib.rs` or `src/main.rs` (or inherit it from `[workspace.lints.rust]`).
2. **Audited FFI Exception List**:
   - `unsafe` blocks are permitted **only** in dedicated, minimally scoped FFI wrapper crates:
     - `umcf-llama-sys` / `umcf-llama-ffi` (`llama.cpp` / `ggml` C/C++ bindings)
     - Platform hardware probe or low-level OS capture/input bindings (`umcf-hw-sys`, `umcf-capture-sys`)
     - Generated `flutter_rust_bridge` glue modules (isolated to bridge modules)
3. **Mandatory `// SAFETY:` Documentation**:
   - Every `unsafe` block and `unsafe fn` in an FFI crate must be preceded by a `// SAFETY:` comment proving:
     - Pointer validity, alignment, and non-null guarantees
     - Lifetime and ownership exclusivity across the C/Rust boundary
     - Thread-safety (`Send`/`Sync`) invariants of underlying C/C++ contexts
     - Absence of unwinding across FFI boundaries (`std::panic::catch_unwind` at every C-callable entry point)

---

## 2. Error Handling Discipline (`thiserror` & `anyhow`)

1. **Library Crates (`thiserror`)**:
   - All library and domain crates must define exhaustive, structured error enums using `thiserror::Error`.
   - Error variants must carry actionable context (node IDs, stream IDs, capability names, threshold values, I/O paths) and implement machine-readable error codes for wire/FFI translation.
2. **Application Entry Points (`anyhow` / `eyre`)**:
   - `anyhow::Result` is restricted to top-level binary entry points (`main.rs`), CLI command runners, and test harnesses.
3. **Zero Unchecked Unwraps**:
   - `.unwrap()` and `.expect()` are forbidden in production code paths (enforced via `#![deny(clippy::unwrap_used, clippy::expect_used)]`).
   - If an invariant is structurally guaranteed (e.g., a compile-time regex or static constant), initialize it via `std::sync::LazyLock` with a documented invariant test or handle the error path explicitly.

---

## 3. Deterministic Serialization & Wire Protocols

1. **Canonical Wire Framing**:
   - Control and data plane wire messages must use versioned envelopes (magic bytes, protocol version, message type, payload length, and authenticated tag/checksum).
2. **Deterministic Encoding**:
   - When computing cryptographic signatures, content-addressed digests (BLAKE3 / SHA-256), or state checkpoints, serialization must be canonical (e.g., deterministic CBOR/Protobuf/bincode with sorted map keys, no floating-point NaN ambiguity, explicit little-endian integer encoding).
3. **Bounded Deserialization**:
   - Never deserialize untrusted network frames without enforcing strict maximum frame length and allocation limits prior to parsing to prevent OOM denial-of-service attacks.

---

## 4. Async Runtime (`tokio`) & Concurrency Bounds

1. **Bounded Channels Everywhere**:
   - `tokio::sync::mpsc::unbounded_channel` is forbidden in production code. Always use bounded channels (`mpsc::channel(capacity)`) with explicit backpressure, drop policies, or timeout handling.
2. **No Blocking in Async Contexts**:
   - Never call blocking I/O, heavy CPU compute (`llama.cpp` inference steps, video encoding, cryptographic key generation, archive compression), or synchronous `std::sync::Mutex` locks across `.await` points on Tokio worker threads.
   - Offload CPU-intensive or FFI-blocking work to `tokio::task::spawn_blocking` or dedicated pinned compute thread pools.
3. **Structured Cancellation & Graceful Shutdown**:
   - All background tasks, QUIC connection loops, and telemetry collectors must listen to a `tokio_util::sync::CancellationToken` and clean up sockets, file descriptors, and GPU/NPU contexts deterministically within a bounded shutdown timeout.

---

## 5. Memory Pressure & Resource Governance

1. **Pre-Allocation Budget Checks**:
   - Before loading GGUF model shards, allocating KV caches, or initializing frame buffers, query the node's memory governor (`umcf-resource-governor`) to verify available physical RAM/VRAM against the user-configured safety watermark.
2. **OOM Prevention on Mobile Devices**:
   - Mobile OS kernels (Android LMKD, iOS Jetsam) aggressively terminate processes that spike memory. Implement streaming shard loading, memory-mapped weights (`mmap`) where supported, and immediate buffer reclamation upon task completion or thermal/memory pressure warnings.

---

## 6. Compiler & Clippy Pedantic Gates

1. **Zero Warnings Policy**:
   - All workspace crates must compile with `RUSTFLAGS="-D warnings"` and pass `cargo clippy --workspace --all-targets --all-features` with zero warnings.
2. **Required Workspace Lints**:
   - Enable and enforce:
     - `clippy::all`
     - `clippy::pedantic` (with only explicitly justified, narrowly scoped allowances)
     - `clippy::unwrap_used` = `"deny"`
     - `clippy::expect_used` = `"deny"`
     - `clippy::panic` = `"deny"`
     - `clippy::todo` = `"deny"`
     - `clippy::unimplemented` = `"deny"`
     - `clippy::dbg_macro` = `"deny"`
     - `clippy::large_futures` = `"warn"`
     - `rust_2018_idioms` = `"deny"`
