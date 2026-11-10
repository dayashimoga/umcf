# Executable Stage Prompt — Stage 4: Distributed Scheduler, Resource Management, Workload Execution and Recovery

> **Document ID:** `STAGE_04_PROMPT`  
> **Stage Code:** `S4` (`S4.P1.SP1` .. `S4.P3.SP3`)  
> **Master Roadmap Reference:** [`docs/implementation/MASTER_ROADMAP.md`](../MASTER_ROADMAP.md)  
> **Execution Environment:** Rootless Podman (`<= 30 GB` Hard Storage Ceiling)

---

## 1. Role & Context

You are the Chief Systems Architect leading **Stage 4 (Distributed Scheduler, Resource Management, Workload Execution and Recovery)** of the Universal Distributed Mobile-First Compute Fabric (**UMCF**).

- **Stage Goal:** Implement the local resource guard and instant user-activity preemption engine (`umcf-guard`), the heterogeneous capability- and bandwidth-aware DAG scheduler (`umcf-scheduler`), and the sandboxed worker execution engine with Φ-accrual failure detection, incremental checkpointing, and speculative retry (`umcf-worker`).
- **Primary Crates & Services Owned/Modified:** `crates/umcf-guard`, `crates/umcf-scheduler`, `crates/umcf-worker`

---

## 2. Prerequisites & Dependencies

- Stage 3 certified (`STAGE_03_CERTIFICATION.json` present)
- Podman disk usage `< 29.0 GiB`
- Rootless Podman quota enforcer (`scripts/podman-quota-check.sh`) active and reporting `< 29.0 GiB`.

---

## 3. Stage Hierarchy & Executable Prompt Index (`3 Phases × 3 Subphases = 9 Subphases`)

Execute the following phases and subphases in strict sequential order:

### Phase 1 (`S4.P1`): Local Resource Guard, Consent Enforcement & Lifecycle Monitor
- **Phase Prompt:** [`S4_P1_PROMPT.md`](S4_P1_PROMPT.md)
  1. **`S4.P1.SP1` — CPU/Memory/VRAM Cgroups & Job Object Quota Enforcer:** [`S4_P1_SP1.md`](S4_P1_SP1.md)
  2. **`S4.P1.SP2` — Thermal Throttling, Battery Guard & Mobile OS Lifecycle Hooks:** [`S4_P1_SP2.md`](S4_P1_SP2.md)
  3. **`S4.P1.SP3` — Interactive User Activity Detector & Instant Preemption:** [`S4_P1_SP3.md`](S4_P1_SP3.md)

### Phase 2 (`S4.P2`): Heterogeneous Resource-Aware DAG Scheduler
- **Phase Prompt:** [`S4_P2_PROMPT.md`](S4_P2_PROMPT.md)
  4. **`S4.P2.SP1` — Capability-Negotiated Constraint Solver & Cost Model:** [`S4_P2_SP1.md`](S4_P2_SP1.md)
  5. **`S4.P2.SP2` — Bandwidth/Latency-Aware Workload Partitioning:** [`S4_P2_SP2.md`](S4_P2_SP2.md)
  6. **`S4.P2.SP3` — Queue Prioritization, Backpressure & Admission Control:** [`S4_P2_SP3.md`](S4_P2_SP3.md)

### Phase 3 (`S4.P3`): Sandboxed Workload Execution, Checkpointing & Fault Recovery
- **Phase Prompt:** [`S4_P3_PROMPT.md`](S4_P3_PROMPT.md)
  7. **`S4.P3.SP1` — Isolated Worker Execution Sandbox & Artifact Staging:** [`S4_P3_SP1.md`](S4_P3_SP1.md)
  8. **`S4.P3.SP2` — Heartbeat Failure Detection, Checkpointing & Speculative Retry:** [`S4_P3_SP2.md`](S4_P3_SP2.md)
  9. **`S4.P3.SP3` — End-to-End Distributed Job Execution Verification:** [`S4_P3_SP3.md`](S4_P3_SP3.md)

---

## 4. Implementation Tasks (Stage-Level Orchestration)

1. Execute Phase `S4.P1` (`SP1 -> SP2 -> SP3`) and generate `S4_P1_PHASE_SUMMARY.json`.
2. Execute Phase `S4.P2` (`SP1 -> SP2 -> SP3`) and generate `S4_P2_PHASE_SUMMARY.json`.
3. Execute Phase `S4.P3` (`SP1 -> SP2 -> SP3`) and generate `S4_P3_PHASE_SUMMARY.json`.
4. Run full Stage 4 verification script `scripts/verify-stage-04.sh` to audit all 9 subphase evidence files, SBOMs, coverage reports, and Podman storage usage.

---

## 5. Acceptance Criteria (Stage-Level Measurable Gates)

- [ ] All 9 subphases (`S4.P1.SP1` through `S4.P3.SP3`) pass their individual acceptance criteria.
- [ ] Workspace compiles with `0` warnings and `0` errors across all Stage 4 targets.
- [ ] Combined line coverage across Stage 4 crates (`crates/umcf-guard`, `crates/umcf-scheduler`, `crates/umcf-worker`) is `>= 85.0%`.
- [ ] `scripts/check-no-placeholders.sh` confirms `0` `todo!()`, `unimplemented!()`, or mock-only stubs.
- [ ] `scripts/podman-quota-check.sh` confirms total container, volume, and `target/` usage is `<= 29.0 GiB` (`< 30 GB` hard ceiling).

---

## 6. Required Tests (Stage Rollup)

- **Unit Tests:** 100% pass rate across all unit tests in `S4.P1` – `S4.P3`.
- **Integration Tests:** Cross-phase integration tests verifying end-to-end data/control flow across `crates/umcf-guard`, `crates/umcf-scheduler`, `crates/umcf-worker`.
- **Simulation Tests:** Chaos, fuzzing (`proptest`), and fault-injection suites for all Stage 4 subsystems.
- **Hardware-Required Tests:** Executed on host hardware or documented with reproducible hardware execution manifests.
- **Manually Verified Tests:** All 9 subphase manual verification checklists signed off.

---

## 7. Required Evidence & Artifacts

- 9 Subphase Evidence Files: `artifacts/evidence/stage-04/S4_P{1..3}_SP{1..3}_evidence.json`
- 3 Phase Summary Files: `artifacts/evidence/stage-04/S4_P{1..3}_PHASE_SUMMARY.json`
- Stage Certification Manifest: `artifacts/evidence/stage-04/STAGE_04_CERTIFICATION.json`
- Stage CycloneDX SBOM & Checksums: `artifacts/sbom/stage-04-cyclonedx.json` and `artifacts/manifests/stage-04-sha256sums.txt`

---

## 8. Documentation, `TODO.md` & `CHANGELOG.md` Updates

- All 9 subphases (`S4.P1.SP1` .. `S4.P3.SP3`) marked `[x]` in `TODO.md`.
- Stage 4 release section finalized in `CHANGELOG.md` under `[0.4.0-stage4]`.

---

## 9. Rollback Criteria & Completion Gate

- **Rollback Criteria:** Any regression in Stage `1..4`, any security/cryptographic invariant failure, or any breach of the `30 GB` Podman disk ceiling triggers rollback to the latest certified phase tag.
- **Completion Gate:** Stage 4 is certified complete when `scripts/verify-stage-04.sh` exits `0` and produces `STAGE_04_CERTIFICATION.json`. Tag repository `v0.4.0-certified`.
