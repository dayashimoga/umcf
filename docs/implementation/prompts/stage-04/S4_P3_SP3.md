# Executable Subphase Prompt — `S4.P3.SP3`: End-to-End Distributed Job Execution Verification

> **Stage:** Stage 4 — Distributed Scheduler, Resource Management, Workload Execution and Recovery (`STAGE_04_PROMPT.md`)  
> **Phase:** Phase `S4.P3` — Sandboxed Workload Execution, Checkpointing & Fault Recovery (`S4_P3_PROMPT.md`)  
> **Subphase:** `S4.P3.SP3` — End-to-End Distributed Job Execution Verification  
> **Execution Mode:** Single-Subphase Atomic Implementation inside Rootless Podman (`<= 30 GB` Hard Disk Ceiling)

---

## 1. Role & Context

You are a Principal Distributed Systems & Cross-Platform Rust/Flutter/Edge Architect executing Subphase **`S4.P3.SP3` (End-to-End Distributed Job Execution Verification)** of the Universal Distributed Mobile-First Compute Fabric (**UMCF**).

- **Subphase Scope:** Integrate `umcf-identity`, `umcf-enrollment`, `umcf-registry`, `umcf-net`, `umcf-hwprobe`, `umcf-guard`, `umcf-scheduler`, and `umcf-worker` into a complete multi-node distributed compute cluster test suite and certify Stage 4.
- **Architectural Mandate:** Implement production-grade, memory-safe, deterministic code with zero `todo!()`, `unimplemented!()`, or mock-only placeholders in production crates/services. All build, lint, and test executions must run inside rootless Podman containers while enforcing the `<= 30 GB` disk ceiling via `scripts/podman-quota-check.sh`.

---

## 2. Prerequisites & Dependencies

- **Upstream Gate:** `S4.P3.SP2` must be marked `[x]` in `TODO.md` with verified evidence JSON on disk.
- `S4.P3.SP2` completed
- **Podman Quota Pre-Check:** Run `scripts/podman-quota-check.sh` before starting implementation; total footprint must be `< 29.0 GiB` (`31,138,512,896` bytes).

---

## 3. Implementation Tasks

Execute the following concrete implementation tasks in order:

1. Implement `ClusterNodeRuntime` in `crates/umcf-worker/src/runtime.rs` wiring discovery, enrollment, registry sync, hardware probing, resource guard, DAG scheduler (when in `Master`/`Hybrid` role), and sandboxed worker execution into a single cohesive node runtime.
2. Implement an end-to-end multi-stage DAG benchmark job (distributed parallel data compression + cryptographic hashing + matrix reduction) executing across a 4-node heterogeneous virtual cluster (`1 Master/Hybrid Desktop`, `2 Worker Laptops`, `1 Mobile Worker`).
3. Implement `scripts/verify-stage-04.sh` validating all 9 Stage 4 subphases (`S4.P1.SP1` .. `S4.P3.SP3`), running chaos fault injection (mid-job mobile thermal throttle + worker kill + user input preemption), checking zero placeholders, and verifying Podman disk footprint `< 29.0 GiB`.

---

## 4. Acceptance Criteria (Measurable)

All criteria below must be quantitatively verified before marking `S4.P3.SP3` complete:

- [ ] A 4-node cluster enrolls, probes hardware, schedules a 24-task DAG, survives 1 mobile thermal eviction + 1 sudden worker kill, and produces bit-exact deterministic final output verified by BLAKE3.
- [ ] End-to-end scheduling + dispatch overhead per task on LAN is `< 15 ms`.
- [ ] `scripts/verify-stage-04.sh` exits `0` and emits `artifacts/evidence/stage-04/STAGE_04_CERTIFICATION.json`.
- [ ] Zero unresolved placeholders (`scripts/check-no-placeholders.sh` exits `0`).
- [ ] Rootless Podman disk usage after build & test remains `< 29.0 GiB` (`scripts/podman-quota-check.sh` exits `0`).

---

## 5. Required Tests (Classified)

Every test category below must be implemented, executed, and logged in the subphase evidence bundle:

- **Unit Tests:**
  - Test `ClusterNodeRuntime` lifecycle startup, role transition (`Worker -> Master` promotion during active DAG), and graceful shutdown.
- **Integration Tests:**
  - Execute the 4-node 24-task DAG end-to-end integration test over real QUIC mTLS sockets.
- **Simulation / Fault-Injection Tests:**
  - Run simultaneous chaos events during the 24-task DAG (kill Master + thermally throttle Worker 3 + inject user input on Worker 2) and verify leader re-election, task migration, and 100% job completion.
- **Hardware-Required Tests:**
  - Executes full multi-node runtime inside rootless Podman container.
- **Manually Verified Tests:**
  - Inspect `STAGE_04_CERTIFICATION.json` and confirm `TODO.md` and `CHANGELOG.md` have all Stage 1–4 items marked `[x]`.

---

## 6. Required Evidence & Artifacts

Generate and archive the following deterministic artifacts before completing `S4.P3.SP3`:

- `artifacts/evidence/stage-04/S4_P3_SP3_evidence.json`
- `artifacts/evidence/stage-04/e2e_distributed_dag_report.json`
- `artifacts/evidence/stage-04/STAGE_04_CERTIFICATION.json`
- Standard Evidence Schema (`artifacts/evidence/stage-04/S4_P3_SP3_evidence.json`):
  ```json
  {
    "subphase": "S4.P3.SP3",
    "title": "End-to-End Distributed Job Execution Verification",
    "git_commit": "<HEAD_SHA>",
    "timestamp_utc": "<ISO-8601>",
    "podman_total_bytes": 0,
    "podman_limit_bytes": 31138512896,
    "placeholder_count": 0,
    "tests": {
      "unit": "PASS",
      "integration": "PASS",
      "simulation": "PASS",
      "hardware_required": "PASS_OR_DEFERRED_WITH_MANIFEST",
      "manually_verified": "PASS"
    },
    "artifact_hashes": {}
  }
  ```

---

## 7. Documentation, `TODO.md` & `CHANGELOG.md` Updates

1. **Module & Rustdoc Documentation:** Document all public structs, traits, functions, error codes, and wire schemas added in `S4.P3.SP3`.
2. **`TODO.md` Update:** Flip `- [ ] S4.P3.SP3` to `- [x] S4.P3.SP3` in `TODO.md` and record the evidence file path.
3. **`CHANGELOG.md` Update:** Append a bullet under `[Unreleased] -> ### Stage 4 — Phase 3` summarizing the concrete deliverables and metrics of `S4.P3.SP3`.

---

## 8. Rollback Criteria & Completion Gate

### Rollback Criteria
Immediately revert `S4.P3.SP3` changes (`git reset --hard` to the `S4.P3.SP2` checkpoint tag) and prune container caches if any of the following occur:
1. `scripts/podman-quota-check.sh` exceeds `29.0 GiB` and automated `cargo sweep` / `podman system prune` cannot recover below `25.0 GiB`.
2. Any prior stage or subphase test suite regresses.
3. Wire protocol backward compatibility in `umcf-proto` is broken without an explicit major version negotiation path.

### Completion Gate
`S4.P3.SP3` is **COMPLETE** if and only if:
- All implementation tasks are committed with zero placeholders;
- Unit, Integration, and Simulation tests pass 100% inside Podman;
- `artifacts/evidence/stage-04/S4_P3_SP3_evidence.json` is written and validated;
- `TODO.md` and `CHANGELOG.md` are updated.
