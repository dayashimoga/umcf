# Executable Subphase Prompt — `S4.P2.SP1`: Capability-Negotiated Constraint Solver & Cost Model

> **Stage:** Stage 4 — Distributed Scheduler, Resource Management, Workload Execution and Recovery (`STAGE_04_PROMPT.md`)  
> **Phase:** Phase `S4.P2` — Heterogeneous Resource-Aware DAG Scheduler (`S4_P2_PROMPT.md`)  
> **Subphase:** `S4.P2.SP1` — Capability-Negotiated Constraint Solver & Cost Model  
> **Execution Mode:** Single-Subphase Atomic Implementation inside Rootless Podman (`<= 30 GB` Hard Disk Ceiling)

---

## 1. Role & Context

You are a Principal Distributed Systems & Cross-Platform Rust/Flutter/Edge Architect executing Subphase **`S4.P2.SP1` (Capability-Negotiated Constraint Solver & Cost Model)** of the Universal Distributed Mobile-First Compute Fabric (**UMCF**).

- **Subphase Scope:** Implement `crates/umcf-scheduler/src/solver.rs` matching DAG `TaskSpec` hard constraints (required `ComputeBackend`, min RAM/VRAM, instruction sets, privacy enclave flag) and scoring candidate nodes using measured `SignedHardwareAttestation` benchmarks, thermal/battery headroom, and network RTT.
- **Architectural Mandate:** Implement production-grade, memory-safe, deterministic code with zero `todo!()`, `unimplemented!()`, or mock-only placeholders in production crates/services. All build, lint, and test executions must run inside rootless Podman containers while enforcing the `<= 30 GB` disk ceiling via `scripts/podman-quota-check.sh`.

---

## 2. Prerequisites & Dependencies

- **Upstream Gate:** `S4.P1.SP3` must be marked `[x]` in `TODO.md` with verified evidence JSON on disk.
- `S4.P1.SP3` completed
- **Podman Quota Pre-Check:** Run `scripts/podman-quota-check.sh` before starting implementation; total footprint must be `< 29.0 GiB` (`31,138,512,896` bytes).

---

## 3. Implementation Tasks

Execute the following concrete implementation tasks in order:

1. Create `crates/umcf-scheduler` and define `JobDag`, `TaskNode`, `TaskEdge` (data dependency size in bytes), and `PlacementConstraint` (`HardRequirements` + `SoftPreferences`).
2. Implement `ConstraintSolver::filter_feasible_nodes(task, cluster_snapshot)` pruning any node that violates `HardwareCapabilityDescriptor`, `ResourceConsentPolicy`, `ThermalBatteryGuard` regime, or `PrivateAiLocalOnly` boundary.
3. Implement `RooflineCostModel::estimate_execution_time_ms(task, node)` combining compute time (`task_flops / node_effective_gflops`), memory time (`task_memory_bytes / node_mem_bandwidth`), data transfer time (`edge_bytes / link_bandwidth + rtt`), and reliability penalty (mobile battery/roaming factor).

---

## 4. Acceptance Criteria (Measurable)

All criteria below must be quantitatively verified before marking `S4.P2.SP1` complete:

- [ ] Constraint solver schedules a 100-task DAG across 32 heterogeneous nodes in `< 10 ms`.
- [ ] Zero tasks are ever assigned to a node that lacks the required `ComputeBackend`, exceeds consented RAM/VRAM, or is in `PausedWaitCoolOrCharge` state (`100%` constraint satisfaction).
- [ ] `RooflineCostModel` accurately prefers a high-VRAM desktop for compute-heavy tasks while preferring local execution over slow WAN links when transfer cost dominates compute savings.
- [ ] Zero unresolved placeholders (`scripts/check-no-placeholders.sh` exits `0`).
- [ ] Rootless Podman disk usage after build & test remains `< 29.0 GiB` (`scripts/podman-quota-check.sh` exits `0`).

---

## 5. Required Tests (Classified)

Every test category below must be implemented, executed, and logged in the subphase evidence bundle:

- **Unit Tests:**
  - Test `filter_feasible_nodes` and `RooflineCostModel` across 15 synthetic node profiles (smartphone, laptop, GPU workstation, relayed WAN node).
- **Integration Tests:**
  - Feed real `SignedHardwareAttestation` records from `umcf-hwprobe` into `ConstraintSolver` and verify optimal placement plan generation.
- **Simulation / Fault-Injection Tests:**
  - Generate 1,000 random DAGs and cluster states via `proptest` and verify every generated `PlacementPlan` satisfies all hard constraints or returns `UnschedulableReason` diagnostics.
- **Hardware-Required Tests:**
  - Not Required.
- **Manually Verified Tests:**
  - Inspect generated `PlacementPlan` JSON explanation and cost breakdown.

---

## 6. Required Evidence & Artifacts

Generate and archive the following deterministic artifacts before completing `S4.P2.SP1`:

- `artifacts/evidence/stage-04/S4_P2_SP1_evidence.json`
- `artifacts/evidence/stage-04/scheduler_placement_explain.json`
- Standard Evidence Schema (`artifacts/evidence/stage-04/S4_P2_SP1_evidence.json`):
  ```json
  {
    "subphase": "S4.P2.SP1",
    "title": "Capability-Negotiated Constraint Solver & Cost Model",
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

1. **Module & Rustdoc Documentation:** Document all public structs, traits, functions, error codes, and wire schemas added in `S4.P2.SP1`.
2. **`TODO.md` Update:** Flip `- [ ] S4.P2.SP1` to `- [x] S4.P2.SP1` in `TODO.md` and record the evidence file path.
3. **`CHANGELOG.md` Update:** Append a bullet under `[Unreleased] -> ### Stage 4 — Phase 2` summarizing the concrete deliverables and metrics of `S4.P2.SP1`.

---

## 8. Rollback Criteria & Completion Gate

### Rollback Criteria
Immediately revert `S4.P2.SP1` changes (`git reset --hard` to the `S4.P1.SP3` checkpoint tag) and prune container caches if any of the following occur:
1. `scripts/podman-quota-check.sh` exceeds `29.0 GiB` and automated `cargo sweep` / `podman system prune` cannot recover below `25.0 GiB`.
2. Any prior stage or subphase test suite regresses.
3. Wire protocol backward compatibility in `umcf-proto` is broken without an explicit major version negotiation path.

### Completion Gate
`S4.P2.SP1` is **COMPLETE** if and only if:
- All implementation tasks are committed with zero placeholders;
- Unit, Integration, and Simulation tests pass 100% inside Podman;
- `artifacts/evidence/stage-04/S4_P2_SP1_evidence.json` is written and validated;
- `TODO.md` and `CHANGELOG.md` are updated.
