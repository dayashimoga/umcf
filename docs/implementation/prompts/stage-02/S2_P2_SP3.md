# Executable Subphase Prompt — `S2.P2.SP3`: Dynamic Master/Worker/Hybrid State Machine & Election

> **Stage:** Stage 2 — Rust Core, Identity, Enrollment, Device Registry and Cluster Management (`STAGE_02_PROMPT.md`)  
> **Phase:** Phase `S2.P2` — QR Enrollment, RBAC & Dynamic Master/Worker Role Engine (`S2_P2_PROMPT.md`)  
> **Subphase:** `S2.P2.SP3` — Dynamic Master/Worker/Hybrid State Machine & Election  
> **Execution Mode:** Single-Subphase Atomic Implementation inside Rootless Podman (`<= 30 GB` Hard Disk Ceiling)

---

## 1. Role & Context

You are a Principal Distributed Systems & Cross-Platform Rust/Flutter/Edge Architect executing Subphase **`S2.P2.SP3` (Dynamic Master/Worker/Hybrid State Machine & Election)** of the Universal Distributed Mobile-First Compute Fabric (**UMCF**).

- **Subphase Scope:** Implement the deterministic node role state machine (`Master`, `Worker`, `Hybrid`, `AutonomousSolo`) and lightweight term-based leader election with fitness scoring (uptime, power state, RAM/CPU headroom, network stability).
- **Architectural Mandate:** Implement production-grade, memory-safe, deterministic code with zero `todo!()`, `unimplemented!()`, or mock-only placeholders in production crates/services. All build, lint, and test executions must run inside rootless Podman containers while enforcing the `<= 30 GB` disk ceiling via `scripts/podman-quota-check.sh`.

---

## 2. Prerequisites & Dependencies

- **Upstream Gate:** `S2.P2.SP2` must be marked `[x]` in `TODO.md` with verified evidence JSON on disk.
- `S2.P2.SP2` completed
- **Podman Quota Pre-Check:** Run `scripts/podman-quota-check.sh` before starting implementation; total footprint must be `< 29.0 GiB` (`31,138,512,896` bytes).

---

## 3. Implementation Tasks

Execute the following concrete implementation tasks in order:

1. Implement `crates/umcf-enrollment/src/role_engine.rs` defining `NodeRoleState` (`Master`, `Worker`, `Hybrid`, `Candidate`, `AutonomousSolo`) and transition rules allowing any node to act as Master, Worker, or both (`Hybrid`).
2. Implement `MasterFitnessScore` calculation in `crates/umcf-enrollment/src/election.rs`: weights AC/charging power (`+400`), non-mobile or docked stability (`+250`), available RAM (`+150`), low packet loss (`+100`), and deterministic tie-breaking via BLAKE3(`term || did`).
3. Implement Raft-inspired term-based leader election (`RequestVote`, `VoteGranted`, `LeaderHeartbeat`) with configurable election timeout (`1500 ms` – `3000 ms` jittered) and graceful leadership step-down (`LeaderTransfer`) when a Master unplugs from AC power or drops below battery threshold.

---

## 4. Acceptance Criteria (Measurable)

All criteria below must be quantitatively verified before marking `S2.P2.SP3` complete:

- [ ] In a 5-node cluster where the active Master disconnects or steps down, a new Master is elected within `<= 3.0 seconds` with zero split-brain leaders in the same quorum term.
- [ ] A single isolated mobile or desktop device automatically enters `AutonomousSolo` (Master+Worker) mode within `500 ms` and seamlessly merges when peers appear.
- [ ] `LeaderTransfer` hands off coordinator state to the highest-fitness peer in `< 250 ms` without dropping active job metadata.
- [ ] Zero unresolved placeholders (`scripts/check-no-placeholders.sh` exits `0`).
- [ ] Rootless Podman disk usage after build & test remains `< 29.0 GiB` (`scripts/podman-quota-check.sh` exits `0`).

---

## 5. Required Tests (Classified)

Every test category below must be implemented, executed, and logged in the subphase evidence bundle:

- **Unit Tests:**
  - Test `MasterFitnessScore` ranking across heterogeneous nodes (charging desktop vs battery phone vs plugged-in laptop).
- **Integration Tests:**
  - Run a 5-node in-memory cluster, kill the elected Master node, and verify deterministic re-election of the next highest-fitness node.
- **Simulation / Fault-Injection Tests:**
  - Simulate simultaneous election timeouts across 7 nodes with 15% packet loss and verify single-leader convergence within 2 terms.
- **Hardware-Required Tests:**
  - Not Required.
- **Manually Verified Tests:**
  - Inspect state transition diagram and election term logs.

---

## 6. Required Evidence & Artifacts

Generate and archive the following deterministic artifacts before completing `S2.P2.SP3`:

- `artifacts/evidence/stage-02/S2_P2_SP3_evidence.json`
- `artifacts/evidence/stage-02/election_failover_trace.json`
- Standard Evidence Schema (`artifacts/evidence/stage-02/S2_P2_SP3_evidence.json`):
  ```json
  {
    "subphase": "S2.P2.SP3",
    "title": "Dynamic Master/Worker/Hybrid State Machine & Election",
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

1. **Module & Rustdoc Documentation:** Document all public structs, traits, functions, error codes, and wire schemas added in `S2.P2.SP3`.
2. **`TODO.md` Update:** Flip `- [ ] S2.P2.SP3` to `- [x] S2.P2.SP3` in `TODO.md` and record the evidence file path.
3. **`CHANGELOG.md` Update:** Append a bullet under `[Unreleased] -> ### Stage 2 — Phase 2` summarizing the concrete deliverables and metrics of `S2.P2.SP3`.

---

## 8. Rollback Criteria & Completion Gate

### Rollback Criteria
Immediately revert `S2.P2.SP3` changes (`git reset --hard` to the `S2.P2.SP2` checkpoint tag) and prune container caches if any of the following occur:
1. `scripts/podman-quota-check.sh` exceeds `29.0 GiB` and automated `cargo sweep` / `podman system prune` cannot recover below `25.0 GiB`.
2. Any prior stage or subphase test suite regresses.
3. Wire protocol backward compatibility in `umcf-proto` is broken without an explicit major version negotiation path.

### Completion Gate
`S2.P2.SP3` is **COMPLETE** if and only if:
- All implementation tasks are committed with zero placeholders;
- Unit, Integration, and Simulation tests pass 100% inside Podman;
- `artifacts/evidence/stage-02/S2_P2_SP3_evidence.json` is written and validated;
- `TODO.md` and `CHANGELOG.md` are updated.
