# Executable Subphase Prompt — `S4.P3.SP2`: Heartbeat Failure Detection, Checkpointing & Speculative Retry

> **Stage:** Stage 4 — Distributed Scheduler, Resource Management, Workload Execution and Recovery (`STAGE_04_PROMPT.md`)  
> **Phase:** Phase `S4.P3` — Sandboxed Workload Execution, Checkpointing & Fault Recovery (`S4_P3_PROMPT.md`)  
> **Subphase:** `S4.P3.SP2` — Heartbeat Failure Detection, Checkpointing & Speculative Retry  
> **Execution Mode:** Single-Subphase Atomic Implementation inside Rootless Podman (`<= 30 GB` Hard Disk Ceiling)

---

## 1. Role & Context

You are a Principal Distributed Systems & Cross-Platform Rust/Flutter/Edge Architect executing Subphase **`S4.P3.SP2` (Heartbeat Failure Detection, Checkpointing & Speculative Retry)** of the Universal Distributed Mobile-First Compute Fabric (**UMCF**).

- **Subphase Scope:** Implement `crates/umcf-worker/src/recovery.rs` and `crates/umcf-scheduler/src/fault_tolerance.rs` providing Φ-accrual heartbeat failure detection, periodic incremental task checkpointing, automatic task migration on worker preemption/disconnect, and speculative backup execution for tail stragglers.
- **Architectural Mandate:** Implement production-grade, memory-safe, deterministic code with zero `todo!()`, `unimplemented!()`, or mock-only placeholders in production crates/services. All build, lint, and test executions must run inside rootless Podman containers while enforcing the `<= 30 GB` disk ceiling via `scripts/podman-quota-check.sh`.

---

## 2. Prerequisites & Dependencies

- **Upstream Gate:** `S4.P3.SP1` must be marked `[x]` in `TODO.md` with verified evidence JSON on disk.
- `S4.P3.SP1` completed
- **Podman Quota Pre-Check:** Run `scripts/podman-quota-check.sh` before starting implementation; total footprint must be `< 29.0 GiB` (`31,138,512,896` bytes).

---

## 3. Implementation Tasks

Execute the following concrete implementation tasks in order:

1. Implement `PhiAccrualDetector` tracking inter-arrival times of worker `TaskHeartbeat` messages (containing progress %, current checkpoint BLAKE3 digest, and thermal/battery state) to compute suspicion level $\phi(t)$ Adaptive to LAN vs WAN jitter.
2. Implement `CheckpointManager` enabling long-running tasks to snapshot intermediate state (`CheckpointBlob` + step index) locally and replicate lightweight checkpoint metadata to the Master/peer storage.
3. Implement automatic failover and speculative execution (`SpeculativeRetryEngine`): when a worker disconnects ($\phi > 8.0$) or emits `NodeGuardEvent::EvacuateNode`, reschedule the task on the next-best healthy peer starting from the latest verified `CheckpointBlob` rather than step 0.

---

## 4. Acceptance Criteria (Measurable)

All criteria below must be quantitatively verified before marking `S4.P3.SP2` complete:

- [ ] Worker crash or ungraceful disconnect is detected within `< 1.5 seconds` on LAN and `< 3.5 seconds` on WAN.
- [ ] Resumed task restores from its latest `CheckpointBlob` and completes without re-computing already-checkpointed steps (`>= 80%` wasted-work reduction on mid-way failure).
- [ ] Idempotent commit barrier guarantees that if both a slow original worker and a speculative backup worker finish, only one canonical output is committed.
- [ ] Zero unresolved placeholders (`scripts/check-no-placeholders.sh` exits `0`).
- [ ] Rootless Podman disk usage after build & test remains `< 29.0 GiB` (`scripts/podman-quota-check.sh` exits `0`).

---

## 5. Required Tests (Classified)

Every test category below must be implemented, executed, and logged in the subphase evidence bundle:

- **Unit Tests:**
  - Test `PhiAccrualDetector` sliding window mean/variance math and `CheckpointManager` serialization/restoration.
- **Integration Tests:**
  - Run a 10-step iterative task that checkpoints at step 5, kill the worker at step 7, and verify Worker B resumes from step 5 and produces the exact same final output hash as an uninterrupted run.
- **Simulation / Fault-Injection Tests:**
  - Simulate a tail straggler worker running `5x` slower than peers; verify `SpeculativeRetryEngine` launches a backup task on an idle peer and commits the first finisher.
- **Hardware-Required Tests:**
  - Not Required.
- **Manually Verified Tests:**
  - Inspect checkpoint recovery timeline and speculative execution trace.

---

## 6. Required Evidence & Artifacts

Generate and archive the following deterministic artifacts before completing `S4.P3.SP2`:

- `artifacts/evidence/stage-04/S4_P3_SP2_evidence.json`
- `artifacts/evidence/stage-04/checkpoint_failover_trace.json`
- Standard Evidence Schema (`artifacts/evidence/stage-04/S4_P3_SP2_evidence.json`):
  ```json
  {
    "subphase": "S4.P3.SP2",
    "title": "Heartbeat Failure Detection, Checkpointing & Speculative Retry",
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

1. **Module & Rustdoc Documentation:** Document all public structs, traits, functions, error codes, and wire schemas added in `S4.P3.SP2`.
2. **`TODO.md` Update:** Flip `- [ ] S4.P3.SP2` to `- [x] S4.P3.SP2` in `TODO.md` and record the evidence file path.
3. **`CHANGELOG.md` Update:** Append a bullet under `[Unreleased] -> ### Stage 4 — Phase 3` summarizing the concrete deliverables and metrics of `S4.P3.SP2`.

---

## 8. Rollback Criteria & Completion Gate

### Rollback Criteria
Immediately revert `S4.P3.SP2` changes (`git reset --hard` to the `S4.P3.SP1` checkpoint tag) and prune container caches if any of the following occur:
1. `scripts/podman-quota-check.sh` exceeds `29.0 GiB` and automated `cargo sweep` / `podman system prune` cannot recover below `25.0 GiB`.
2. Any prior stage or subphase test suite regresses.
3. Wire protocol backward compatibility in `umcf-proto` is broken without an explicit major version negotiation path.

### Completion Gate
`S4.P3.SP2` is **COMPLETE** if and only if:
- All implementation tasks are committed with zero placeholders;
- Unit, Integration, and Simulation tests pass 100% inside Podman;
- `artifacts/evidence/stage-04/S4_P3_SP2_evidence.json` is written and validated;
- `TODO.md` and `CHANGELOG.md` are updated.
