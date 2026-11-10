# ENGINEERING CONTRACT — UNIVERSAL MOBILE COMPUTE FABRIC (UMCF)

**Document Version:** `1.0.0`  
**Status:** Enforced / Mandatory  
**Applies To:** All human engineers, Antigravity agents, subagents, CI/CD pipelines, and release gates.

---

## 1. Mission & Architectural Mandate

The **Universal Mobile Compute Fabric (UMCF)** is a decentralized, cross-platform compute fabric enabling Android, iOS, Windows, Linux, macOS, and server nodes to dynamically operate as **Masters (Coordinators)**, **Workers**, or **Hybrid (Master + Worker)** nodes across LAN and WAN topologies.

### 1.1 Core Architectural Invariants
1. **Strict Control-Plane vs. Data-Plane Separation**:
   - **Control Plane**: Handles identity attestation, enrollment, cluster membership, capability advertisement, scheduling metadata, and cryptographic accounting receipts. Operates via **Local Autonomous Coordinator** (LAN/offline-first), **Cloudflare Workers + Durable Objects** (global edge primary), or **Google Cloud Run** (stateless container fallback).
   - **Data Plane**: Handles bulk tensor/activation transfers, model weights, GPU/media buffers, remote desktop/game video streams, and content-addressed storage chunks over direct multiplexed **P2P QUIC (mTLS)** connections with STUN/ICE NAT traversal and encrypted TURN/DERP relay fallback. Bulk payloads MUST NEVER traverse the Cloudflare/Cloud Run control plane.
2. **Explicit Heterogeneity (No Fake Resource Pooling)**:
   - Heterogeneous CPUs (x86_64, ARM64), GPUs (Vulkan, Metal, DX12, CUDA, ROCm, WebGPU), NPUs (Intel NPU, Apple Neural Engine, Qualcomm Hexagon/NNAPI), RAM, and operating systems CANNOT be transparently pooled as uniform shared-memory hardware.
   - All distributed execution MUST use explicit capability negotiation, deterministic protocol framing, and architecture-aware workload partitioning (e.g., layer/pipeline partitioning for `llama.cpp` inference, tile/frame partitioning for rendering, segment partitioning for media transcoding).
3. **Offline & Local-First Operation**:
   - Every cluster operation (discovery, enrollment, scheduling, distributed AI inference, rendering, streaming, local storage) MUST work autonomously on an isolated LAN without internet connectivity.
4. **Podman-First Reproducible Environment (30 GB Cap)**:
   - Host installations are optional convenience layers; all builds, lints, and tests are reproducible via Podman containers (`infra/podman/`).
   - Total Podman storage allocation MUST NOT exceed **30 GB (`32,212,254,720` bytes)**. Automated pre-flight guards (`scripts/enforce-podman-quota.ps1` and `scripts/enforce-podman-quota.sh`) enforce this invariant before container builds.

---

## 2. Non-Negotiable Engineering Rules

1. **Zero Fake / Placeholder Production Code**:
   - No `todo!()`, `unimplemented!()`, hardcoded mock return values, dummy sleep timers pretending to compute, or stubbed UI handlers in completed features.
2. **Zero Decorative UI Controls**:
   - Every button, toggle, slider, menu item, and action in the Flutter mobile/desktop apps and Svelte 5 web dashboard MUST be wired to real, tested state and runtime actions.
3. **Zero Silent Error Suppression**:
   - Never ignore `Result` or `Future` failures (`let _ = ...` on fallible I/O is forbidden). Every error must be typed, logged with structured context, and propagated or handled via explicit recovery logic.
4. **Zero Unverified Hardware Claims**:
   - Hardware acceleration (Vulkan, Metal, DX12, CUDA, NPU) is reported as active ONLY when probed from the real OS/driver runtime and verified via an actual compute execution pass.
   - When physical hardware is absent in CI or a developer machine, tests MUST be classified as `Simulation` or `Hardware-Unverified` — never certified as `Hardware-Verified`.
5. **Versioned Protocol Evolution**:
   - Every wire message in `umcf-proto` carries a semantic `ProtocolVersion { major, minor, patch }` and capability bitmask. Breaking changes require a major version bump and explicit migration/compatibility negotiation.
6. **Explicit Consent & Device Safety**:
   - Workers execute remote tasks ONLY within user-approved resource envelopes (max CPU %, max RAM/VRAM MB, storage quota MB, charging-only policy, thermal ceiling °C, and idle-only preemption rules).
7. **Append-Only Historical Records**:
   - `TODO.md` and `CHANGELOG.md` are strictly append-only audit trails. Completed tasks in `TODO.md` are marked `[x]` with completion timestamps and commit/stage references; historical entries are never deleted.

---

## 3. Language & Component Standards

### 3.1 Rust Core (`crates/*`)
- **Edition & Toolchain**: Rust 2021 / stable toolchain (`>= 1.85`).
- **Safety**: `#![deny(unsafe_code)]` enforced across all pure-Rust crates (`umcf-proto`, `umcf-core`, `umcf-coordinator`, `umcf-worker`, `umcf-scheduler`, `umcf-cli`, `umcf-daemon`). Any FFI wrapper crate (e.g., `llama.cpp` bindings in Stage 5) must isolate `unsafe` blocks with documented `// SAFETY:` invariants and Miri/asan verification.
- **Linting**: `cargo fmt --all -- --check` and `cargo clippy --workspace --all-targets -- -D warnings` must pass with zero warnings.
- **Error Handling**: Library crates use `thiserror` with domain-specific error enums and protocol error codes; binary crates (`umcf-cli`, `umcf-daemon`) use structured error reporting with actionable exit codes.

### 3.2 Flutter Mobile & Desktop (`apps/mobile-desktop`)
- **Targets**: Android (APK/AAB), iOS (Archive/IPA), Windows (x64 portable/installer), Linux (x64 AppImage/bundle), macOS (Universal `.app`).
- **Standards**: `flutter analyze --no-pub` with zero issues; strict null safety; responsive adaptive layout breakpoints (`<600px` compact mobile, `600-1024px` tablet, `>1024px` desktop split-pane).

### 3.3 Svelte 5 Web Dashboard (`apps/web-dashboard`) & Cloudflare Control Plane (`services/control-plane-cf`)
- **Standards**: Svelte 5 runes (`$state`, `$derived`, `$effect`), TypeScript `strict: true`, zero `any` types without explicit justification, WCAG 2.1 AA keyboard/ARIA accessibility.

---

## 4. Quality Gates, Coverage & Test Classification

### 4.1 Coverage & Pass Rate Mandate
- **Required Test Pass Rate**: **100%** across all active test suites.
- **Meaningful Code Coverage**: **>= 95%** line/branch coverage on implemented domain and runtime modules.

### 4.2 Mandatory Test Classification Taxonomy
Every automated or manual test MUST be tagged with one of the following classifications:
1. **`[UNIT]`**: Deterministic, isolated in-memory test of types, state machines, parsers, and scheduling algorithms.
2. **`[INTEGRATION]`**: Multi-component test verifying real IPC, loopback QUIC networking, SQLite/storage persistence, CLI invocation, or control-plane API contracts.
3. **`[SIMULATION]`**: Multi-node cluster topology, network fault injection, latency/jitter emulation, or simulated heterogeneous capability profiles.
4. **`[HARDWARE-REQUIRED]`**: Tests requiring physical GPU (Vulkan/Metal/DX12/CUDA), NPU, battery/thermal sensors, or multi-device LAN setup. If hardware is unavailable, the test runner records `SKIPPED_HARDWARE_UNAVAILABLE` and marks the feature `Hardware-Unverified`.
5. **`[MANUALLY-VERIFIED]`**: Human-verified UX, QR camera scanning, or physical plug-pull recovery with captured artifact evidence.

---

## 5. Security & Cryptographic Baseline

1. **Node Identity**: Ed25519 identity keypairs (`NodeId` derived from BLAKE3-256 / SHA-256 fingerprint of the public key).
2. **Transport Security**: Mutual TLS (mTLS 1.3) over QUIC or Noise_XX handshake with X25519 ephemeral key exchange and ChaCha20-Poly1305 / AES-256-GCM authenticated encryption.
3. **Enrollment**: Short-lived (<= 300 seconds), single-use cryptographic enrollment tokens encoded in QR payloads or CLI join tokens with mutual challenge-response verification.
4. **Zero Secret Leakage**: Pre-commit and CI checks prohibit private keys, tokens, `.p12`, `.jks`, or credentials in source control or CI logs.

---

## 6. Subphase Execution & Definition of Done (DoD)

Every subphase (`S{1..8}.P{1..3}.SP{1..3}`) must execute the 10-step workflow:
1. Read `ENGINEERING_CONTRACT.md` and relevant `.agent/skills/`.
2. Inspect affected crates, apps, and dependency contracts.
3. Implement the complete, production-grade feature.
4. Integrate into the runtime, CLI, and UI/API surfaces.
5. Execute `[UNIT]`, `[INTEGRATION]`, `[SIMULATION]`, and available `[HARDWARE-REQUIRED]` tests.
6. Perform security, memory-pressure, performance, and regression audits.
7. Remediate all defects and warnings.
8. Update architecture/user docs and append to `TODO.md` and `CHANGELOG.md`.
9. Capture reproducible verification evidence in `docs/certification/`.
10. Commit validated changes only when all gates are green.

