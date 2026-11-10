---
name: umcf-testing-verification
description: >-
  Guides comprehensive testing, coverage analysis (>=95%), five-tier test classification (Unit, Integration, Simulation, Hardware-Required, Manually-Verified), chaos mesh simulation, and verification evidence generation for UMCF. Use when writing or running tests across Rust, Flutter, and Svelte.
---

# UMCF Testing, Coverage & Verification Skill

Use this skill when designing test suites, measuring coverage, running multi-node simulations, or assembling verification evidence across the UMCF workspace.

---

## 1. Mandatory Quality Gates

1. **100% Pass Rate**:
   - Zero failing tests across Rust, Flutter, Svelte, and Control Plane worker suites.
   - Zero flaky tests: tests must use deterministic clocks/seeds (`tokio::time::pause()` or seeded RNGs) where appropriate, or bounded local loopback retries.
2. **>= 95% Meaningful Coverage**:
   - Measure Rust line/branch coverage via `cargo llvm-cov --workspace --all-features`.
   - Measure Flutter coverage via `flutter test --coverage`.
   - Measure Svelte/TypeScript coverage via `vitest run --coverage`.
   - Every module must test happy paths, boundary conditions, malformed wire frames, timeout/cancellation paths, and permission denials.

---

## 2. Five-Tier Test Classification & Tagging

Tag every test module or test case clearly with its tier:

| Tier | Tag Convention | Execution Environment | Purpose |
| :--- | :--- | :--- | :--- |
| **1. Unit** | `[Unit]` / `unit_*` | Pure in-memory, any host | Test algorithms, crypto, parsers, schedulers, state stores |
| **2. Integration** | `[Integration]` / `int_*` | Localhost UDP/QUIC loopback, temp SQLite, FFI | Test multi-crate interaction, real QUIC mTLS handshakes, end-to-end RPCs |
| **3. Simulation** | `[Simulation]` / `sim_*` | Parameterized virtual mesh (`umcf-sim`) | Test 2 to 64 heterogeneous nodes with injected latency, packet loss, battery drain, and mid-token node churn |
| **4. Hardware-Required** | `[Hardware-Required]` / `hw_*` | Real CPU/GPU/NPU + real GGUF weights | Certify real hardware probing, `llama.cpp` inference, and hardware media encode/decode |
| **5. Manually-Verified** | `[Manually-Verified]` | Multi-device physical setup | Documented human verification runbook with recorded logs/screenshots |

---

## 3. Writing High-Value UMCF Tests

1. **Protocol & Security Negative Tests**:
   - Corrupted magic bytes, truncated frames, oversized payload length headers (`u32::MAX`), expired capability tokens, replayed enrollment nonces, and mismatched peer certificates must all return specific typed errors without panicking.
2. **Distributed Chaos Simulation Tests**:
   - Create a 3-node virtual pipeline (`Phone-A` -> `Laptop-B` -> `Desktop-C`) running a 12-layer model (4 layers each).
   - Inject `ThermalState::Serious` on `Phone-A` at token step 5 and assert seamless migration to `Standby-D` with zero lost tokens and verified output continuity.
3. **UI Completeness Tests**:
   - In Flutter (`WidgetTester`) and Svelte (`@testing-library/svelte` / `Playwright`), programmatically enumerate interactive buttons/toggles on each view, trigger them, and assert that the underlying state store or FFI mock/bridge receives the expected call.
