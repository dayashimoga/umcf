---
trigger: always_on
description: Global non-negotiable engineering governance and quality standards for the Universal Mobile Compute Fabric (UMCF).
---

# 00 - Global Engineering Governance (UMCF)

This rule defines the non-negotiable global engineering standards for the **Universal Mobile Compute Fabric (UMCF)** repository. Every agent, subagent, and contributor must enforce these mandates across all crates, mobile/desktop applications, web dashboards, control-plane workers, and Podman infrastructure.

---

## 1. Zero Placeholder, Dummy, or Stub Code

1. **No Unimplemented Macros or Panics in Production Paths**:
   - `todo!()`, `unimplemented!()`, `unreachable!()` (without mathematical/type-level proof), and dummy return values (`Ok(Default::default())`, hardcoded mock JSON, fake telemetry numbers) are strictly forbidden in non-test targets.
2. **Real Execution Only**:
   - Every API endpoint, CLI subcommand, RPC handler, FFI function, and background worker must execute real domain logic backed by actual state, storage, network transport, or hardware probing.
3. **No Silent Fallbacks to Fake Data**:
   - When an external dependency or hardware accelerator is unavailable, return a structured, typed diagnostic error or transition to an explicitly labeled degraded state—never return fabricated success metrics or simulated outputs disguised as real execution.

---

## 2. Zero Decorative UI Controls

1. **Every Interactive Control Must Be Wired**:
   - Across both the Flutter mobile/desktop apps and the Svelte 5 web dashboard, every button, toggle, slider, dropdown, menu item, modal action, and keyboard shortcut must be bound to a real state transition, FFI call, or authenticated API/WebSocket/QUIC action.
2. **No Dead Actions or "Coming Soon" Toasts**:
   - Controls that trigger empty callbacks (`() {}`), `console.log`, or placeholder snackbars are prohibited. If a feature is gated by hardware capability or node role, disable the control with an explicit, accurate diagnostic tooltip explaining the prerequisite capability.
3. **End-to-End State Reflection**:
   - UI state must reflect real-time backend state via reactive streams (`flutter_rust_bridge` streams, Riverpod providers, or Svelte 5 runes backed by WebSocket/WebTransport telemetry) with proper loading, error, and retry states.

---

## 3. Strict Separation of Control Plane and Data Plane

1. **Control Plane Scope**:
   - The Cloudflare Workers / Durable Objects control plane (and the Cloud Run containerized fallback) is strictly restricted to:
     - Mesh discovery and peer coordination
     - Short-lived enrollment token issuance and mTLS/Noise identity brokering
     - NAT traversal signaling (STUN/TURN coordination, ICE/QUIC candidate exchange)
     - Cluster topology metadata, capability advertisements, and coarse health heartbeats
2. **Data Plane Scope**:
   - High-bandwidth, latency-sensitive, or privacy-sensitive payloads—including `llama.cpp` tensor/activation shards, model weights, raw compute payloads, remote desktop/game video-audio frames, input events, and bulk file transfers—must flow **exclusively over the encrypted P2P QUIC data plane** (or explicit user-authorized relay when direct P2P is impossible).
3. **Architectural Enforcement**:
   - Never route tensor activations, raw video streams, or unencrypted user payloads through control-plane HTTP/WebSocket endpoints.

---

## 4. Explicit Hardware Capability Negotiation & Honesty

1. **Probed Capabilities Only**:
   - Every node must probe its actual CPU topology, instruction sets (AVX2, AVX-512, NEON, SVE), GPU APIs (Vulkan, Metal, DirectX 12, CUDA, OpenCL), NPU runtimes (Android NNAPI/QNN, Apple CoreML/ANE, ONNX Runtime EP), RAM availability, battery state, and thermal headroom before advertising capabilities to the mesh.
2. **Never Claim Unverified Hardware Acceleration**:
   - If a GPU or NPU backend fails initialization, lacks required operator support, or has not been verified on the host device, the runtime must report the backend as `Unavailable` or `Unsupported` with an exact reason code, and explicitly label any CPU fallback as `CPU_FALLBACK`.
3. **No Fake Transparent Memory or GPU Pooling**:
   - UMCF operates across heterogeneous mobile, desktop, and server nodes connected over variable networks. Never present distributed nodes as a single cache-coherent NUMA memory space or unified single-GPU VRAM pool.
   - Distributed execution must always be modeled explicitly as **pipeline/tensor stage partitioning**, **sharded inference**, or **task-level scheduling** with explicit network serialization, latency accounting, and backpressure.

---

## 5. Testing, Coverage, and Pass-Rate Gates

1. **100% Test Pass Rate**:
   - All unit, integration, simulation, and UI test suites across Rust (`cargo nextest` / `cargo test`), Flutter (`flutter test`), and Svelte/TypeScript (`vitest`, `playwright`) must pass at **100%** with zero flaky or ignored tests (`#[ignore]` is permitted only for `[Hardware-Required]` tests gated by feature/env flags and documented in the hardware verification matrix).
2. **>= 95% Meaningful Line & Branch Coverage**:
   - Maintain a minimum of **95% line coverage** across core logic crates, state management, protocol state machines, scheduler algorithms, security primitives, and UI view-models.
   - Coverage tests must assert behavioral invariants, boundary conditions, failure recovery, and malicious/malformed input rejection—not superficial getter/setter calls.

---

## 6. Append-Only Documentation & Traceability (`TODO.md` and `CHANGELOG.md`)

1. **Append-Only `TODO.md`**:
   - Never delete historical task entries or rewrite history in `TODO.md`. Mark completed items with `[x]`, completion timestamps, and verification references; append newly discovered tasks or subphase breakdowns cleanly.
2. **Append-Only `CHANGELOG.md`**:
   - Every completed subphase or significant change must append a structured entry to `CHANGELOG.md` detailing Added, Changed, Fixed, Security, and Verification Evidence (test counts, coverage metrics, and artifact hashes).
3. **Synchronized Architecture & Contract Docs**:
   - Whenever wire protocols, FFI signatures, capability schemas, or CLI flags change, update the corresponding specification documents in the same commit.
