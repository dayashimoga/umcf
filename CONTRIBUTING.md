# Contributing to Universal Mobile Compute Fabric (UMCF)

Thank you for contributing to the **Universal Mobile Compute Fabric (UMCF)**! To keep UMCF secure, deterministic, and deployable across Android, iOS, Windows, Linux, macOS, Web, and Server environments, all contributions must adhere to the engineering standards and verification gates outlined below.

---

## 1. Architectural Non-Negotiables

Before opening a pull request or implementing a feature, verify that your design respects our core Architecture Decision Records (`docs/architecture/adr/`):

1. **Strict Control Plane vs. Data Plane Separation ([ADR-0002](file:///g:/ucf/docs/architecture/adr/ADR-0002-control-vs-data-plane-separation.md))**:
   - Never route model weights, intermediate tensors, video/audio frames, or CAS file chunks through `services/control-plane-cf` or `services/control-plane-cloudrun`.
   - Control Plane payloads are strictly capped at `64 KiB` (`MAX_CONTROL_FRAME_BYTES`).
2. **No Transparent Shared-Memory Pooling Claims ([ADR-0003](file:///g:/ucf/docs/architecture/adr/ADR-0003-heterogeneous-compute-no-transparent-pooling.md))**:
   - Workloads must decompose into explicit DAG stages with well-defined memory, backend capability, and bandwidth contracts in `umcf-scheduler`.
   - Always preserve the **Single-Node Colocation Preference Rule** when a single node can satisfy the job's memory and latency SLOs faster than splitting over the network.
3. **Mobile Hardware Safety First (`NFR-014`, `NFR-018`–`NFR-020`)**:
   - Never allocate more than `75%` of available system RAM on Android/iOS.
   - Never disable or bypass the `43°C` thermal cutoff or `< 20%` unplugged battery circuit breakers.
4. **30 GB Podman Disk Footprint Cap ([ADR-0006](file:///g:/ucf/docs/architecture/adr/ADR-0006-podman-30gb-reproducible-dev-environment.md))**:
   - Do not introduce heavy container images or enable full DWARF `debug = 2` symbols in workspace profiles.

---

## 2. Code Quality & Safety Standards

### 2.1 Rust Standards (`crates/*` and `services/control-plane-cloudrun`)
- **Zero `unsafe` in Pure-Rust Crates**: `#[forbid(unsafe_code)]` is mandatory in `umcf-proto`, `umcf-core` (outside OS keystore FFI), `umcf-scheduler`, and `umcf-coordinator`. Any `unsafe` block in `umcf-ffi` or `umcf-engine-*` MUST be preceded by a rigorous `// SAFETY:` invariant comment.
- **No Panics on Untrusted Input (`NFR-032`)**: Never call `.unwrap()`, `.expect()`, or unchecked slice indexing `buf[i]` on network packets, peer messages, GGUF headers, or user inputs. Always return typed `Result<T, E>` using `thiserror`.
- **Bounded Concurrency (`NFR-017`)**: `tokio::sync::mpsc::unbounded_channel` is forbidden. Every channel and queue must have an explicit capacity bound and backpressure handling.
- **Secret Hygiene (`NFR-027`)**: Any struct holding private keys, shared secrets, enrollment nonces, or capability JWTs must derive `ZeroizeOnDrop` and redact sensitive fields in its `fmt::Debug` implementation.

### 2.2 TypeScript / Svelte 5 Standards (`services/control-plane-cf`, `apps/web-dashboard`)
- Strict TypeScript mode (`"strict": true`, no implicit `any`).
- Full WCAG 2.1 AA accessibility compliance (semantic HTML, ARIA labels, keyboard navigation).

### 2.3 Flutter / Dart Standards (`apps/mobile-desktop`)
- All heavy compute, cryptography, and network I/O must execute on Rust worker threads via `flutter_rust_bridge` asynchronous streams—never block the UI isolate (`< 16.6ms` frame budget).

---

## 3. Requirement Traceability & Commit Conventions

Every pull request and commit must link to the specific **Functional (`FR-xxx`)** or **Non-Functional (`NFR-xxx`)** requirement and **Roadmap Stage/Phase/Subphase (`Sx.Py.SPz`)** it implements or fixes (see [`docs/requirements/TRACEABILITY_MATRIX.md`](file:///g:/ucf/docs/requirements/TRACEABILITY_MATRIX.md)).

### Commit Message Format (Conventional Commits)
```text
<type>(<scope>): <concise summary> [Stage/Phase/Subphase] [Req-IDs]

Detailed explanation of architectural changes, invariants preserved, and verification performed.
```

---

## 4. Verification & Testing Checklist

Before submitting a PR:
- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
- [ ] `cargo test --workspace` (100% pass rate, >= 95% coverage)
- [ ] `bash scripts/enforce-podman-quota.sh` (or `pwsh scripts/enforce-podman-quota.ps1`) exits 0
- [ ] `bash scripts/check-no-placeholders.sh` exits 0
- [ ] `TODO.md` updated (append-only)
- [ ] `CHANGELOG.md` updated (append-only)
