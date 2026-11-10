---
description: Security, concurrency, error-handling, UI completeness, and cross-component code review checklist for UMCF.
---

# UMCF Code Review & Quality Gate Workflow

Run this checklist before completing any subphase or merging any change across the UMCF codebase.

---

## 1. Rust Core & Systems Checklist

- [ ] **Safety & Lints**: Does every pure-Rust crate have `#![forbid(unsafe_code)]`? Do all crates pass `cargo clippy --workspace --all-targets --all-features -- -D warnings` with zero warnings?
- [ ] **Audited FFI (`*-sys`)**: Is every `unsafe` block in `umcf-llama-sys` or platform FFI crates documented with a rigorous `// SAFETY:` proof and guarded against unwinding across FFI boundaries (`catch_unwind`)?
- [ ] **Error Handling**: Are `.unwrap()`, `.expect()`, `panic!()`, `todo!()`, and `unimplemented!()` completely absent from production code paths?
- [ ] **Async & Concurrency**:
  - Are all channels bounded (`mpsc::channel(N)`) with explicit backpressure handling?
  - Are `std::sync::Mutex` locks never held across `.await` points?
  - Are CPU-intensive operations (`llama.cpp` decode, hashing, compression) offloaded via `spawn_blocking` or dedicated compute threads?
  - Does every long-running task respect `CancellationToken` on shutdown?
- [ ] **Memory & Thermal Governance**: Does the code verify available memory against `umcf-resource-governor` watermarks before allocating model shards or frame buffers, and react immediately to thermal/battery preemption signals?

---

## 2. Security & Privacy Checklist

- [ ] **Identity & Transport**: Are all P2P QUIC and control-plane connections mutually authenticated (Ed25519/X25519 mTLS or Noise) with zero plaintext fallbacks?
- [ ] **Capability Authorization**: Does every incoming RPC and stream validate a signed, non-expired, non-revoked `CapabilityToken` scoped to the requested operation and resource quota?
- [ ] **Input & Frame Validation**: Do wire deserializers and GGUF parsers enforce strict byte-length caps and checked arithmetic (`checked_mul`/`checked_add`) before allocating memory?
- [ ] **Consent & Revocation**: Does remote compute or desktop streaming check active local user consent, and does the emergency kill switch terminate active sessions within < 500 ms?
- [ ] **Zero Secret Leakage**: Are private keys wrapped in `Zeroizing`, and are tokens, keys, and user prompts redacted from `Debug` formatting and `tracing` logs?

---

## 3. Flutter Mobile/Desktop UI Checklist

- [ ] **Zero Decorative Controls**: Is every button, toggle, slider, and menu action wired to a real Riverpod `Notifier` method and FFI call?
- [ ] **Async State Coverage**: Does every `AsyncValue<T>` in the UI properly render `data`, `loading`, and actionable `error` states?
- [ ] **Responsive Breakpoints**: Does the screen adapt cleanly across Compact (<600px phone), Medium (600–1024px tablet), and Expanded (>1024px desktop) viewports without overflow errors?
- [ ] **UI Thread Smoothness**: Are all FFI calls asynchronous so the main Dart UI isolate never blocks?

---

## 4. Svelte 5 Web Dashboard Checklist

- [ ] **Svelte 5 Runes**: Does the code use `$state`, `$state.raw`, `$derived`, `$effect`, and `$props` exclusively (zero legacy `export let` or `$:` statements)?
- [ ] **Strict TypeScript**: Does `svelte-check` pass with zero errors and zero explicit `any` types?
- [ ] **Accessibility**: Do all interactive elements have keyboard navigation support, focus indicators, and appropriate ARIA attributes?

---

## 5. Cross-Component Contract Synchronization

- [ ] Are wire protocol changes in `umcf-proto` synchronized across `umcf-node`, `umcf-ffi`, `apps/flutter_client`, `apps/web_dashboard`, and `services/control_plane_worker`?
- [ ] Are `TODO.md` and `CHANGELOG.md` updated in append-only fashion?
