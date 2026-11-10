# Executable Subphase Prompt — `S4.P2.SP3`: Queue Prioritization, Backpressure & Admission Control

> **Stage:** Stage 4 — Distributed Scheduler, Resource Management, Workload Execution and Recovery (`STAGE_04_PROMPT.md`)  
> **Phase:** Phase `S4.P2` — Heterogeneous Resource-Aware DAG Scheduler (`S4_P2_PROMPT.md`)  
> **Subphase:** `S4.P2.SP3` — Queue Prioritization, Backpressure & Admission Control  
> **Execution Mode:** Single-Subphase Atomic Implementation inside Rootless Podman (`<= 30 GB` Hard Disk Ceiling)

---

## 1. Role & Context

You are a Principal Distributed Systems & Cross-Platform Rust/Flutter/Edge Architect executing Subphase **`S4.P2.SP3` (Queue Prioritization, Backpressure & Admission Control)** of the Universal Distributed Mobile-First Compute Fabric (**UMCF**).

- **Subphase Scope:** Implement `crates/umcf-scheduler/src/queue.rs` with multi-priority Deficit Round-Robin (DRR) scheduling (`InteractiveRealtime`, `HighPriorityUser`, `StandardBatch`, `BackgroundMaintenance`), aging to prevent starvation, and token-based backpressure/admission control.
- **Architectural Mandate:** Implement production-grade, memory-safe, deterministic code with zero `todo!()`, `unimplemented!()`, or mock-only placeholders in production crates/services. All build, lint, and test executions must run inside rootless Podman containers while enforcing the `<= 30 GB` disk ceiling via `scripts/podman-quota-check.sh`.

---

## 2. Prerequisites & Dependencies

- **Upstream Gate:** `S4.P2.SP2` must be marked `[x]` in `TODO.md` with verified evidence JSON on disk.
- `S4.P2.SP2` completed
- **Podman Quota Pre-Check:** Run `scripts/podman-quota-check.sh` before starting implementation; total footprint must be `< 29.0 GiB` (`31,138,512,896` bytes).

---

## 3. Implementation Tasks

Execute the following concrete implementation tasks in order:

1. Implement `AdmissionController` enforcing max queued jobs, max cluster memory commitment, and per-requester fair-share quotas (`ErrorCode::E4101_CLUSTER_OVERLOADED` with `Retry-After` estimate when saturated).
2. Implement `PriorityJobQueue` with 4 priority bands, anti-starvation priority aging (promoting `StandardBatch` jobs waited `> 60s`), and gang-scheduling reservation for multi-node pipeline jobs (e.g., multi-node LLM inference).
3. Wire end-to-end backpressure from `umcf-worker` queues back to `umcf-scheduler` and `umcf-net`.

---

## 4. Acceptance Criteria (Measurable)

All criteria below must be quantitatively verified before marking `S4.P2.SP3` complete:

- [ ] `InteractiveRealtime` jobs bypass queued `StandardBatch` jobs and begin scheduling within `< 15 ms`.
- [ ] Anti-starvation aging guarantees low-priority jobs make progress even under continuous high-priority job arrivals.
- [ ] Deadlock-free gang scheduling: concurrent multi-node gang reservation requests never deadlock when cluster capacity is partially contended.
- [ ] Zero unresolved placeholders (`scripts/check-no-placeholders.sh` exits `0`).
- [ ] Rootless Podman disk usage after build & test remains `< 29.0 GiB` (`scripts/podman-quota-check.sh` exits `0`).

---

## 5. Required Tests (Classified)

Every test category below must be implemented, executed, and logged in the subphase evidence bundle:

- **Unit Tests:**
  - Test Deficit Round-Robin quantum fairness, priority aging promotion, and admission control rejection thresholds.
- **Integration Tests:**
  - Submit 50 `StandardBatch` jobs followed by 5 `InteractiveRealtime` jobs and verify all 5 interactive jobs schedule ahead of remaining batch tasks.
- **Simulation / Fault-Injection Tests:**
  - Submit 20 concurrent gang-scheduled 3-node jobs onto a 4-node cluster and verify zero reservation deadlocks across 1,000 randomized runs.
- **Hardware-Required Tests:**
  - Not Required.
- **Manually Verified Tests:**
  - Inspect queue depth, wait-time p50/p99, and admission rejection metrics.

---

## 6. Required Evidence & Artifacts

Generate and archive the following deterministic artifacts before completing `S4.P2.SP3`:

- `artifacts/evidence/stage-04/S4_P2_SP3_evidence.json`
- `artifacts/evidence/stage-04/queue_fairness_and_backpressure.json`
- Standard Evidence Schema (`artifacts/evidence/stage-04/S4_P2_SP3_evidence.json`):
  ```json
  {
    "subphase": "S4.P2.SP3",
    "title": "Queue Prioritization, Backpressure & Admission Control",
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

1. **Module & Rustdoc Documentation:** Document all public structs, traits, functions, error codes, and wire schemas added in `S4.P2.SP3`.
2. **`TODO.md` Update:** Flip `- [ ] S4.P2.SP3` to `- [x] S4.P2.SP3` in `TODO.md` and record the evidence file path.
3. **`CHANGELOG.md` Update:** Append a bullet under `[Unreleased] -> ### Stage 4 — Phase 2` summarizing the concrete deliverables and metrics of `S4.P2.SP3`.

---

## 8. Rollback Criteria & Completion Gate

### Rollback Criteria
Immediately revert `S4.P2.SP3` changes (`git reset --hard` to the `S4.P2.SP2` checkpoint tag) and prune container caches if any of the following occur:
1. `scripts/podman-quota-check.sh` exceeds `29.0 GiB` and automated `cargo sweep` / `podman system prune` cannot recover below `25.0 GiB`.
2. Any prior stage or subphase test suite regresses.
3. Wire protocol backward compatibility in `umcf-proto` is broken without an explicit major version negotiation path.

### Completion Gate
`S4.P2.SP3` is **COMPLETE** if and only if:
- All implementation tasks are committed with zero placeholders;
- Unit, Integration, and Simulation tests pass 100% inside Podman;
- `artifacts/evidence/stage-04/S4_P2_SP3_evidence.json` is written and validated;
- `TODO.md` and `CHANGELOG.md` are updated.
