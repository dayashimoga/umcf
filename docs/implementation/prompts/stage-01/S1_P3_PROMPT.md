# Executable Phase Prompt — `S1.P3`: Multi-Platform CI/CD Pipelines, SBOM, Checksums & Baseline Verification

> **Stage:** Stage 1 — Engineering Foundation, Repository and Development Infrastructure (`STAGE_01_PROMPT.md`)  
> **Phase:** `S1.P3` — Multi-Platform CI/CD Pipelines, SBOM, Checksums & Baseline Verification  
> **Subphases:** `S1.P3.SP1`, `S1.P3.SP2`, `S1.P3.SP3`  
> **Target Environment:** Rootless Podman (`<= 30 GB` Hard Disk Ceiling)

---

## 1. Role & Context

You are the Lead Systems Architect executing Phase **`S1.P3` (Multi-Platform CI/CD Pipelines, SBOM, Checksums & Baseline Verification)** of Stage 1 in the Universal Distributed Mobile-First Compute Fabric (**UMCF**).

- **Phase Objective:** Establish automated CI pipelines for Rust, Web, Control Plane, and Flutter, enforce code coverage and lint gates, and build the CycloneDX SBOM and SHA-256 release manifest generator.
- **Execution Discipline:** Execute `S1.P3.SP1`, `S1.P3.SP2`, and `S1.P3.SP3` sequentially. Do not begin a subsequent subphase until the current subphase's Completion Gate and Evidence JSON are verified.

---

## 2. Prerequisites & Dependencies

- **Upstream Dependency:** `S1.P2` must be 100% complete and certified.
- **Target Crates / Services:** `crates/umcf-proto, crates/umcf-core, crates/umcf-telemetry`
- **Quota Enforcement:** `scripts/podman-quota-check.sh` must pass before and after each subphase (`< 29.0 GiB`).

---

## 3. Implementation Tasks & Subphase Execution Sequence

Execute the three constituent subphases in strict sequential order:

### 3.1 Subphase `S1.P3.SP1`: Rust/Web/Control-Plane CI & Coverage Gates
- **Prompt File:** [`S1_P3_SP1.md`](S1_P3_SP1.md)
- **Scope:** Create GitHub Actions / local Podman CI runners for Rust workspace, Svelte 5 web console, and Cloudflare/Cloud Run control plane with >= 85% line coverage enforcement.
- **Key Deliverables:**
  - Create `.github/workflows/ci-core.yml` and local equivalent `scripts/ci-core-podman.sh` running `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo nextest run --workspace`, and `cargo llvm-cov --workspace --json`.
  - Configure web & edge CI steps in `scripts/ci-web-podman.sh` executing `pnpm --filter web_console lint`, `pnpm --filter web_console test:unit`, and `pnpm --filter cf_control_plane test`.
  - Embed `scripts/podman-quota-check.sh` as both pre-step and post-step in every CI runner script.

### 3.2 Subphase `S1.P3.SP2`: Flutter Multi-OS CI & Artifact Packaging
- **Prompt File:** [`S1_P3_SP2.md`](S1_P3_SP2.md)
- **Scope:** Configure Flutter static analysis, widget/unit testing, rust_bridge FFI bindings generator stub, and multi-OS build verification pipeline (`apps/flutter_client`).
- **Key Deliverables:**
  - Create `.github/workflows/ci-flutter.yml` and `scripts/ci-flutter-podman.sh` executing `flutter analyze --fatal-infos --fatal-warnings` and `flutter test --coverage`.
  - Implement typed Dart models in `apps/flutter_client/lib/src/models/hardware_capability.dart` matching `schemas/hardware_capability_v1.schema.json`.
  - Create initial status dashboard widget in `apps/flutter_client/lib/src/ui/foundation_status_screen.dart` displaying protocol version, node readiness, and theme adaptation (mobile/desktop breakpoints).

### 3.3 Subphase `S1.P3.SP3`: Release Manifest, SBOM Generation & Stage 1 Certification
- **Prompt File:** [`S1_P3_SP3.md`](S1_P3_SP3.md)
- **Scope:** Implement deterministic CycloneDX SBOM generation, SHA-256/BLAKE3 artifact manifest signing, `cargo-deny` license/advisory auditing, and the automated Stage 1 gate verifier.
- **Key Deliverables:**
  - Create `deny.toml` configuring `cargo-deny` for license allow-listing (`MIT`, `Apache-2.0`, `BSD-3-Clause`, `ISC`, `Unicode-3.0`, `Zlib`), banned crates, and RustSec advisory checks.
  - Implement `scripts/generate-sbom-and-manifest.sh` generating `artifacts/sbom/stage-01-cyclonedx.json` and `artifacts/manifests/stage-01-sha256sums.txt`.
  - Implement `scripts/verify-stage-01.sh` validating all 9 Stage 1 subphase evidence files, running `podman-quota-check.sh`, `check-no-placeholders.sh`, `cargo deny check`, and emitting `artifacts/evidence/stage-01/STAGE_01_CERTIFICATION.json`.

---

## 4. Acceptance Criteria (Phase-Level Measurable Gates)

- [ ] All acceptance criteria in `S1_P3_SP1.md`, `S1_P3_SP2.md`, and `S1_P3_SP3.md` are satisfied.
- [ ] Cross-subphase integration between `S1.P3.SP1`, `S1.P3.SP2`, and `S1.P3.SP3` compiles cleanly with zero warnings (`cargo clippy --workspace --all-targets -- -D warnings`).
- [ ] Line coverage across crates modified in `S1.P3` is `>= 85.0%`.
- [ ] Zero `todo!()`, `unimplemented!()`, or placeholder stubs exist in the codebase (`scripts/check-no-placeholders.sh` exits `0`).
- [ ] Podman storage utilization remains strictly `< 29.0 GiB`.

---

## 5. Required Tests (Classified Rollup)

- **Unit Tests:** All unit test suites across `S1.P3.SP1`, `S1.P3.SP2`, and `S1.P3.SP3` pass via `cargo nextest run`.
- **Integration Tests:** End-to-end phase integration tests verifying `Rust/Web/Control-Plane CI & Coverage Gates` + `Flutter Multi-OS CI & Artifact Packaging` + `Release Manifest, SBOM Generation & Stage 1 Certification` interoperation.
- **Simulation Tests:** Fault-injection, corrupted input, and concurrency stress tests defined in `S1.P3.SP1..SP3`.
- **Hardware-Required Tests:** Executed on target hardware or recorded in hardware execution manifests where containerized hardware passthrough is unavailable.
- **Manually Verified Tests:** Developer visual/CLI inspection checklists completed for `S1.P3.SP1..SP3`.

---

## 6. Required Evidence & Artifacts

- `artifacts/evidence/stage-01/S1_P3_SP1_evidence.json`
- `artifacts/evidence/stage-01/S1_P3_SP2_evidence.json`
- `artifacts/evidence/stage-01/S1_P3_SP3_evidence.json`
- `artifacts/evidence/stage-01/S1_P3_PHASE_SUMMARY.json` aggregating all 3 subphase evidence bundles and phase-level coverage metrics.

---

## 7. Documentation, `TODO.md` & `CHANGELOG.md` Updates

- Verify `S1.P3.SP1`, `S1.P3.SP2`, and `S1.P3.SP3` are all checked `[x]` in `TODO.md`.
- Consolidate `S1.P3` capabilities into `CHANGELOG.md` and module-level architecture docs.

---

## 8. Rollback Criteria & Completion Gate

- **Rollback Criteria:** If phase integration tests fail or Podman disk quota exceeds `29.0 GiB` after cache pruning, roll back to the start of `S1.P3` (`v0.1.2-certified`) and isolate the failing subphase.
- **Completion Gate:** Tag repository checkpoint `v0.1.3-certified` once `S1_P3_PHASE_SUMMARY.json` reports `"status": "PASS"`.
