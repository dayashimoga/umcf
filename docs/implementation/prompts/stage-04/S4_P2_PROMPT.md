# Executable Phase Prompt — `S4.P2`: Heterogeneous Resource-Aware DAG Scheduler

> **Stage:** Stage 4 — Distributed Scheduler, Resource Management, Workload Execution and Recovery (`STAGE_04_PROMPT.md`)  
> **Phase:** `S4.P2` — Heterogeneous Resource-Aware DAG Scheduler  
> **Subphases:** `S4.P2.SP1`, `S4.P2.SP2`, `S4.P2.SP3`  
> **Target Environment:** Rootless Podman (`<= 30 GB` Hard Disk Ceiling)

---

## 1. Role & Context

You are the Lead Systems Architect executing Phase **`S4.P2` (Heterogeneous Resource-Aware DAG Scheduler)** of Stage 4 in the Universal Distributed Mobile-First Compute Fabric (**UMCF**).

- **Phase Objective:** Build `crates/umcf-scheduler` to compile multi-stage jobs into Directed Acyclic Graphs (DAGs), solve multi-dimensional hardware/consent constraints, partition workloads across heterogeneous mobile/desktop/server nodes using a Roofline + network transfer cost model, and enforce fair queuing and backpressure.
- **Execution Discipline:** Execute `S4.P2.SP1`, `S4.P2.SP2`, and `S4.P2.SP3` sequentially. Do not begin a subsequent subphase until the current subphase's Completion Gate and Evidence JSON are verified.

---

## 2. Prerequisites & Dependencies

- **Upstream Dependency:** `S4.P1` must be 100% complete and certified.
- **Target Crates / Services:** `crates/umcf-guard, crates/umcf-scheduler, crates/umcf-worker`
- **Quota Enforcement:** `scripts/podman-quota-check.sh` must pass before and after each subphase (`< 29.0 GiB`).

---

## 3. Implementation Tasks & Subphase Execution Sequence

Execute the three constituent subphases in strict sequential order:

### 3.1 Subphase `S4.P2.SP1`: Capability-Negotiated Constraint Solver & Cost Model
- **Prompt File:** [`S4_P2_SP1.md`](S4_P2_SP1.md)
- **Scope:** Implement `crates/umcf-scheduler/src/solver.rs` matching DAG `TaskSpec` hard constraints (required `ComputeBackend`, min RAM/VRAM, instruction sets, privacy enclave flag) and scoring candidate nodes using measured `SignedHardwareAttestation` benchmarks, thermal/battery headroom, and network RTT.
- **Key Deliverables:**
  - Create `crates/umcf-scheduler` and define `JobDag`, `TaskNode`, `TaskEdge` (data dependency size in bytes), and `PlacementConstraint` (`HardRequirements` + `SoftPreferences`).
  - Implement `ConstraintSolver::filter_feasible_nodes(task, cluster_snapshot)` pruning any node that violates `HardwareCapabilityDescriptor`, `ResourceConsentPolicy`, `ThermalBatteryGuard` regime, or `PrivateAiLocalOnly` boundary.
  - Implement `RooflineCostModel::estimate_execution_time_ms(task, node)` combining compute time (`task_flops / node_effective_gflops`), memory time (`task_memory_bytes / node_mem_bandwidth`), data transfer time (`edge_bytes / link_bandwidth + rtt`), and reliability penalty (mobile battery/roaming factor).

### 3.2 Subphase `S4.P2.SP2`: Bandwidth/Latency-Aware Workload Partitioning
- **Prompt File:** [`S4_P2_SP2.md`](S4_P2_SP2.md)
- **Scope:** Implement `crates/umcf-scheduler/src/partitioner.rs` supporting uneven data-parallel, pipeline-parallel, and tile/segment-parallel workload slicing proportional to each node's effective compute-to-network ratio.
- **Key Deliverables:**
  - Implement `AdaptivePartitioner` calculating non-uniform work slices $w_i \propto \text{Speed}_i$ such that total completion time $\max_i (T_{\text{transfer}}(w_i) + T_{\text{compute}}(w_i))$ is minimized across slow and fast workers (straggler mitigation).
  - Implement `DataLocalityOptimizer` co-locating downstream reduce/stitch tasks on the node that already holds the largest intermediate tensor/media shard.
  - Implement dynamic slice rebalancing (`WorkStealingCoordinator`) when a fast node finishes its assigned slices early while a slower peer still has `>= 2` unstarted slices in its queue.

### 3.3 Subphase `S4.P2.SP3`: Queue Prioritization, Backpressure & Admission Control
- **Prompt File:** [`S4_P2_SP3.md`](S4_P2_SP3.md)
- **Scope:** Implement `crates/umcf-scheduler/src/queue.rs` with multi-priority Deficit Round-Robin (DRR) scheduling (`InteractiveRealtime`, `HighPriorityUser`, `StandardBatch`, `BackgroundMaintenance`), aging to prevent starvation, and token-based backpressure/admission control.
- **Key Deliverables:**
  - Implement `AdmissionController` enforcing max queued jobs, max cluster memory commitment, and per-requester fair-share quotas (`ErrorCode::E4101_CLUSTER_OVERLOADED` with `Retry-After` estimate when saturated).
  - Implement `PriorityJobQueue` with 4 priority bands, anti-starvation priority aging (promoting `StandardBatch` jobs waited `> 60s`), and gang-scheduling reservation for multi-node pipeline jobs (e.g., multi-node LLM inference).
  - Wire end-to-end backpressure from `umcf-worker` queues back to `umcf-scheduler` and `umcf-net`.

---

## 4. Acceptance Criteria (Phase-Level Measurable Gates)

- [ ] All acceptance criteria in `S4_P2_SP1.md`, `S4_P2_SP2.md`, and `S4_P2_SP3.md` are satisfied.
- [ ] Cross-subphase integration between `S4.P2.SP1`, `S4.P2.SP2`, and `S4.P2.SP3` compiles cleanly with zero warnings (`cargo clippy --workspace --all-targets -- -D warnings`).
- [ ] Line coverage across crates modified in `S4.P2` is `>= 85.0%`.
- [ ] Zero `todo!()`, `unimplemented!()`, or placeholder stubs exist in the codebase (`scripts/check-no-placeholders.sh` exits `0`).
- [ ] Podman storage utilization remains strictly `< 29.0 GiB`.

---

## 5. Required Tests (Classified Rollup)

- **Unit Tests:** All unit test suites across `S4.P2.SP1`, `S4.P2.SP2`, and `S4.P2.SP3` pass via `cargo nextest run`.
- **Integration Tests:** End-to-end phase integration tests verifying `Capability-Negotiated Constraint Solver & Cost Model` + `Bandwidth/Latency-Aware Workload Partitioning` + `Queue Prioritization, Backpressure & Admission Control` interoperation.
- **Simulation Tests:** Fault-injection, corrupted input, and concurrency stress tests defined in `S4.P2.SP1..SP3`.
- **Hardware-Required Tests:** Executed on target hardware or recorded in hardware execution manifests where containerized hardware passthrough is unavailable.
- **Manually Verified Tests:** Developer visual/CLI inspection checklists completed for `S4.P2.SP1..SP3`.

---

## 6. Required Evidence & Artifacts

- `artifacts/evidence/stage-04/S4_P2_SP1_evidence.json`
- `artifacts/evidence/stage-04/S4_P2_SP2_evidence.json`
- `artifacts/evidence/stage-04/S4_P2_SP3_evidence.json`
- `artifacts/evidence/stage-04/S4_P2_PHASE_SUMMARY.json` aggregating all 3 subphase evidence bundles and phase-level coverage metrics.

---

## 7. Documentation, `TODO.md` & `CHANGELOG.md` Updates

- Verify `S4.P2.SP1`, `S4.P2.SP2`, and `S4.P2.SP3` are all checked `[x]` in `TODO.md`.
- Consolidate `S4.P2` capabilities into `CHANGELOG.md` and module-level architecture docs.

---

## 8. Rollback Criteria & Completion Gate

- **Rollback Criteria:** If phase integration tests fail or Podman disk quota exceeds `29.0 GiB` after cache pruning, roll back to the start of `S4.P2` (`v0.4.1-certified`) and isolate the failing subphase.
- **Completion Gate:** Tag repository checkpoint `v0.4.2-certified` once `S4_P2_PHASE_SUMMARY.json` reports `"status": "PASS"`.
