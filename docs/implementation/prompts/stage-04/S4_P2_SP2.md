# Executable Subphase Prompt — `S4.P2.SP2`: Bandwidth/Latency-Aware Workload Partitioning

> **Stage:** Stage 4 — Distributed Scheduler, Resource Management, Workload Execution and Recovery (`STAGE_04_PROMPT.md`)  
> **Phase:** Phase `S4.P2` — Heterogeneous Resource-Aware DAG Scheduler (`S4_P2_PROMPT.md`)  
> **Subphase:** `S4.P2.SP2` — Bandwidth/Latency-Aware Workload Partitioning  
> **Execution Mode:** Single-Subphase Atomic Implementation inside Rootless Podman (`<= 30 GB` Hard Disk Ceiling)

---

## 1. Role & Context

You are a Principal Distributed Systems & Cross-Platform Rust/Flutter/Edge Architect executing Subphase **`S4.P2.SP2` (Bandwidth/Latency-Aware Workload Partitioning)** of the Universal Distributed Mobile-First Compute Fabric (**UMCF**).

- **Subphase Scope:** Implement `crates/umcf-scheduler/src/partitioner.rs` supporting uneven data-parallel, pipeline-parallel, and tile/segment-parallel workload slicing proportional to each node's effective compute-to-network ratio.
- **Architectural Mandate:** Implement production-grade, memory-safe, deterministic code with zero `todo!()`, `unimplemented!()`, or mock-only placeholders in production crates/services. All build, lint, and test executions must run inside rootless Podman containers while enforcing the `<= 30 GB` disk ceiling via `scripts/podman-quota-check.sh`.

---

## 2. Prerequisites & Dependencies

- **Upstream Gate:** `S4.P2.SP1` must be marked `[x]` in `TODO.md` with verified evidence JSON on disk.
- `S4.P2.SP1` completed
- **Podman Quota Pre-Check:** Run `scripts/podman-quota-check.sh` before starting implementation; total footprint must be `< 29.0 GiB` (`31,138,512,896` bytes).

---

## 3. Implementation Tasks

Execute the following concrete implementation tasks in order:

1. Implement `AdaptivePartitioner` calculating non-uniform work slices $w_i \propto \text{Speed}_i$ such that total completion time $\max_i (T_{\text{transfer}}(w_i) + T_{\text{compute}}(w_i))$ is minimized across slow and fast workers (straggler mitigation).
2. Implement `DataLocalityOptimizer` co-locating downstream reduce/stitch tasks on the node that already holds the largest intermediate tensor/media shard.
3. Implement dynamic slice rebalancing (`WorkStealingCoordinator`) when a fast node finishes its assigned slices early while a slower peer still has `>= 2` unstarted slices in its queue.

---

## 4. Acceptance Criteria (Measurable)

All criteria below must be quantitatively verified before marking `S4.P2.SP2` complete:

- [ ] Across a heterogeneous cluster (`1x 10-TFLOPS Desktop` + `1x 2-TFLOPS Laptop` + `1x 0.5-TFLOPS Phone`), `AdaptivePartitioner` achieves `>= 1.18x` speedup over single-node Desktop and `>= 3.2x` speedup over uniform 1/3 partitioning (which stalls on the slowest node).
- [ ] `WorkStealingCoordinator` reduces tail straggler idle time by `>= 60%` under dynamic load variance.
- [ ] When network transfer cost to a remote node exceeds the parallel compute gain, the partitioner automatically keeps the workload on the local/fast subset of nodes.
- [ ] Zero unresolved placeholders (`scripts/check-no-placeholders.sh` exits `0`).
- [ ] Rootless Podman disk usage after build & test remains `< 29.0 GiB` (`scripts/podman-quota-check.sh` exits `0`).

---

## 5. Required Tests (Classified)

Every test category below must be implemented, executed, and logged in the subphase evidence bundle:

- **Unit Tests:**
  - Test linear-programming / water-filling slice ratio calculator across unequal node speeds and link bandwidths.
- **Integration Tests:**
  - Execute a multi-slice parallel job across 3 simulated nodes with `20:4:1` compute ratio and verify all nodes finish within `< 10%` wall-clock skew.
- **Simulation / Fault-Injection Tests:**
  - Inject mid-job `50%` slowdown on Worker 2 and verify `WorkStealingCoordinator` steals remaining unstarted slices to Worker 1.
- **Hardware-Required Tests:**
  - Not Required.
- **Manually Verified Tests:**
  - Review Gantt timeline chart data generated from partitioned DAG execution.

---

## 6. Required Evidence & Artifacts

Generate and archive the following deterministic artifacts before completing `S4.P2.SP2`:

- `artifacts/evidence/stage-04/S4_P2_SP2_evidence.json`
- `artifacts/evidence/stage-04/heterogeneous_partition_speedup.json`
- Standard Evidence Schema (`artifacts/evidence/stage-04/S4_P2_SP2_evidence.json`):
  ```json
  {
    "subphase": "S4.P2.SP2",
    "title": "Bandwidth/Latency-Aware Workload Partitioning",
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

1. **Module & Rustdoc Documentation:** Document all public structs, traits, functions, error codes, and wire schemas added in `S4.P2.SP2`.
2. **`TODO.md` Update:** Flip `- [ ] S4.P2.SP2` to `- [x] S4.P2.SP2` in `TODO.md` and record the evidence file path.
3. **`CHANGELOG.md` Update:** Append a bullet under `[Unreleased] -> ### Stage 4 — Phase 2` summarizing the concrete deliverables and metrics of `S4.P2.SP2`.

---

## 8. Rollback Criteria & Completion Gate

### Rollback Criteria
Immediately revert `S4.P2.SP2` changes (`git reset --hard` to the `S4.P2.SP1` checkpoint tag) and prune container caches if any of the following occur:
1. `scripts/podman-quota-check.sh` exceeds `29.0 GiB` and automated `cargo sweep` / `podman system prune` cannot recover below `25.0 GiB`.
2. Any prior stage or subphase test suite regresses.
3. Wire protocol backward compatibility in `umcf-proto` is broken without an explicit major version negotiation path.

### Completion Gate
`S4.P2.SP2` is **COMPLETE** if and only if:
- All implementation tasks are committed with zero placeholders;
- Unit, Integration, and Simulation tests pass 100% inside Podman;
- `artifacts/evidence/stage-04/S4_P2_SP2_evidence.json` is written and validated;
- `TODO.md` and `CHANGELOG.md` are updated.
