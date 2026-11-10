# Executable Stage Prompt — Stage 1: Engineering Foundation, Repository and Development Infrastructure

> **Document ID:** `STAGE_01_PROMPT`  
> **Stage Code:** `S1` (`S1.P1.SP1` .. `S1.P3.SP3`)  
> **Master Roadmap Reference:** [`docs/implementation/MASTER_ROADMAP.md`](../MASTER_ROADMAP.md)  
> **Execution Environment:** Rootless Podman (`<= 30 GB` Hard Storage Ceiling)

---

## 1. Role & Context

You are the Chief Systems Architect leading **Stage 1 (Engineering Foundation, Repository and Development Infrastructure)** of the Universal Distributed Mobile-First Compute Fabric (**UMCF**).

- **Stage Goal:** Establish repository governance, rootless Podman reproducible container toolchains strictly capped under 30 GB total disk footprint, the multi-language workspace skeleton (Rust, Flutter, Svelte 5, Cloudflare Workers, Cloud Run), versioned wire envelopes (`umcf-proto`), core domain descriptors (`umcf-core`), OpenTelemetry tracing/error taxonomy (`umcf-telemetry`), and multi-OS CI/SBOM pipelines.
- **Primary Crates & Services Owned/Modified:** `crates/umcf-proto`, `crates/umcf-core`, `crates/umcf-telemetry`

---

## 2. Prerequisites & Dependencies

- Clean Git repository initialized at `g:\ucf`
- Podman >= 4.9 installed in rootless mode on host
- Host filesystem with >= 35 GB free space (30 GB hard project ceiling)
- Rootless Podman quota enforcer (`scripts/podman-quota-check.sh`) active and reporting `< 29.0 GiB`.

---

## 3. Stage Hierarchy & Executable Prompt Index (`3 Phases × 3 Subphases = 9 Subphases`)

Execute the following phases and subphases in strict sequential order:

### Phase 1 (`S1.P1`): Repository Governance, Monorepo Workspace & Podman 30GB Reproducible Environment
- **Phase Prompt:** [`S1_P1_PROMPT.md`](S1_P1_PROMPT.md)
  1. **`S1.P1.SP1` — Git/Governance/Rules/Skills Bootstrap:** [`S1_P1_SP1.md`](S1_P1_SP1.md)
  2. **`S1.P1.SP2` — Podman 30GB Container Toolchains & Quota Enforcer:** [`S1_P1_SP2.md`](S1_P1_SP2.md)
  3. **`S1.P1.SP3` — Multi-Language Workspace Skeleton & Build Orchestration:** [`S1_P1_SP3.md`](S1_P1_SP3.md)

### Phase 2 (`S1.P2`): Versioned Wire Protocol, Core Domain Types & Capability Schemas
- **Phase Prompt:** [`S1_P2_PROMPT.md`](S1_P2_PROMPT.md)
  4. **`S1.P2.SP1` — `umcf-proto` Envelope & Semantic Versioning Negotiation:** [`S1_P2_SP1.md`](S1_P2_SP1.md)
  5. **`S1.P2.SP2` — `umcf-core` Hardware & Resource Descriptors:** [`S1_P2_SP2.md`](S1_P2_SP2.md)
  6. **`S1.P2.SP3` — Structured Telemetry, Tracing & Error Taxonomy:** [`S1_P2_SP3.md`](S1_P2_SP3.md)

### Phase 3 (`S1.P3`): Multi-Platform CI/CD Pipelines, SBOM, Checksums & Baseline Verification
- **Phase Prompt:** [`S1_P3_PROMPT.md`](S1_P3_PROMPT.md)
  7. **`S1.P3.SP1` — Rust/Web/Control-Plane CI & Coverage Gates:** [`S1_P3_SP1.md`](S1_P3_SP1.md)
  8. **`S1.P3.SP2` — Flutter Multi-OS CI & Artifact Packaging:** [`S1_P3_SP2.md`](S1_P3_SP2.md)
  9. **`S1.P3.SP3` — Release Manifest, SBOM Generation & Stage 1 Certification:** [`S1_P3_SP3.md`](S1_P3_SP3.md)

---

## 4. Implementation Tasks (Stage-Level Orchestration)

1. Execute Phase `S1.P1` (`SP1 -> SP2 -> SP3`) and generate `S1_P1_PHASE_SUMMARY.json`.
2. Execute Phase `S1.P2` (`SP1 -> SP2 -> SP3`) and generate `S1_P2_PHASE_SUMMARY.json`.
3. Execute Phase `S1.P3` (`SP1 -> SP2 -> SP3`) and generate `S1_P3_PHASE_SUMMARY.json`.
4. Run full Stage 1 verification script `scripts/verify-stage-01.sh` to audit all 9 subphase evidence files, SBOMs, coverage reports, and Podman storage usage.

---

## 5. Acceptance Criteria (Stage-Level Measurable Gates)

- [ ] All 9 subphases (`S1.P1.SP1` through `S1.P3.SP3`) pass their individual acceptance criteria.
- [ ] Workspace compiles with `0` warnings and `0` errors across all Stage 1 targets.
- [ ] Combined line coverage across Stage 1 crates (`crates/umcf-proto`, `crates/umcf-core`, `crates/umcf-telemetry`) is `>= 85.0%`.
- [ ] `scripts/check-no-placeholders.sh` confirms `0` `todo!()`, `unimplemented!()`, or mock-only stubs.
- [ ] `scripts/podman-quota-check.sh` confirms total container, volume, and `target/` usage is `<= 29.0 GiB` (`< 30 GB` hard ceiling).

---

## 6. Required Tests (Stage Rollup)

- **Unit Tests:** 100% pass rate across all unit tests in `S1.P1` – `S1.P3`.
- **Integration Tests:** Cross-phase integration tests verifying end-to-end data/control flow across `crates/umcf-proto`, `crates/umcf-core`, `crates/umcf-telemetry`.
- **Simulation Tests:** Chaos, fuzzing (`proptest`), and fault-injection suites for all Stage 1 subsystems.
- **Hardware-Required Tests:** Executed on host hardware or documented with reproducible hardware execution manifests.
- **Manually Verified Tests:** All 9 subphase manual verification checklists signed off.

---

## 7. Required Evidence & Artifacts

- 9 Subphase Evidence Files: `artifacts/evidence/stage-01/S1_P{1..3}_SP{1..3}_evidence.json`
- 3 Phase Summary Files: `artifacts/evidence/stage-01/S1_P{1..3}_PHASE_SUMMARY.json`
- Stage Certification Manifest: `artifacts/evidence/stage-01/STAGE_01_CERTIFICATION.json`
- Stage CycloneDX SBOM & Checksums: `artifacts/sbom/stage-01-cyclonedx.json` and `artifacts/manifests/stage-01-sha256sums.txt`

---

## 8. Documentation, `TODO.md` & `CHANGELOG.md` Updates

- All 9 subphases (`S1.P1.SP1` .. `S1.P3.SP3`) marked `[x]` in `TODO.md`.
- Stage 1 release section finalized in `CHANGELOG.md` under `[0.1.0-stage1]`.

---

## 9. Rollback Criteria & Completion Gate

- **Rollback Criteria:** Any regression in Stage `1..1`, any security/cryptographic invariant failure, or any breach of the `30 GB` Podman disk ceiling triggers rollback to the latest certified phase tag.
- **Completion Gate:** Stage 1 is certified complete when `scripts/verify-stage-01.sh` exits `0` and produces `STAGE_01_CERTIFICATION.json`. Tag repository `v0.1.0-certified`.
