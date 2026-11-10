# Executable Phase Prompt — `S4.P3`: Sandboxed Workload Execution, Checkpointing & Fault Recovery

> **Stage:** Stage 4 — Distributed Scheduler, Resource Management, Workload Execution and Recovery (`STAGE_04_PROMPT.md`)  
> **Phase:** `S4.P3` — Sandboxed Workload Execution, Checkpointing & Fault Recovery  
> **Subphases:** `S4.P3.SP1`, `S4.P3.SP2`, `S4.P3.SP3`  
> **Target Environment:** Rootless Podman (`<= 30 GB` Hard Disk Ceiling)

---

## 1. Role & Context

You are the Lead Systems Architect executing Phase **`S4.P3` (Sandboxed Workload Execution, Checkpointing & Fault Recovery)** of Stage 4 in the Universal Distributed Mobile-First Compute Fabric (**UMCF**).

- **Phase Objective:** Implement `crates/umcf-worker` providing isolated Wasmtime/process task sandboxes with capability-based I/O, BLAKE3 content-addressed artifact staging, Φ-accrual heartbeat failure detection, incremental checkpointing, and automatic speculative retry/failover.
- **Execution Discipline:** Execute `S4.P3.SP1`, `S4.P3.SP2`, and `S4.P3.SP3` sequentially. Do not begin a subsequent subphase until the current subphase's Completion Gate and Evidence JSON are verified.

---

## 2. Prerequisites & Dependencies

- **Upstream Dependency:** `S4.P2` must be 100% complete and certified.
- **Target Crates / Services:** `crates/umcf-guard, crates/umcf-scheduler, crates/umcf-worker`
- **Quota Enforcement:** `scripts/podman-quota-check.sh` must pass before and after each subphase (`< 29.0 GiB`).

---

## 3. Implementation Tasks & Subphase Execution Sequence

Execute the three constituent subphases in strict sequential order:

### 3.1 Subphase `S4.P3.SP1`: Isolated Worker Execution Sandbox & Artifact Staging
- **Prompt File:** [`S4_P3_SP1.md`](S4_P3_SP1.md)
- **Scope:** Implement `crates/umcf-worker/src/sandbox.rs` executing untrusted/portable compute tasks inside a capability-restricted Wasmtime (`wasm32-wasip1`) sandbox or isolated child process with an ephemeral per-task scratch directory and BLAKE3-verified input/output artifact staging.
- **Key Deliverables:**
  - Create `crates/umcf-worker` and implement `WasmSandboxRunner` using `wasmtime` with fuel/epoch interruption (wired to `InstantPreemptor`), bounded linear memory (wired to `QuotaEnforcer`), disabled ambient host filesystem/network access, and explicit preopened scratch dir (`/task/in`, `/task/out`, `/task/checkpoint`).
  - Implement `NativeKernelRunner` for verified built-in native engines (Stage 5 `llama.cpp`, Stage 6 `wgpu`/media) gated strictly by cryptographic engine registry allow-list.
  - Implement `ArtifactStager` streaming input blobs over QUIC, verifying BLAKE3 root hashes before task start, and atomically sealing output artifacts with BLAKE3 digests + execution receipts.

### 3.2 Subphase `S4.P3.SP2`: Heartbeat Failure Detection, Checkpointing & Speculative Retry
- **Prompt File:** [`S4_P3_SP2.md`](S4_P3_SP2.md)
- **Scope:** Implement `crates/umcf-worker/src/recovery.rs` and `crates/umcf-scheduler/src/fault_tolerance.rs` providing Φ-accrual heartbeat failure detection, periodic incremental task checkpointing, automatic task migration on worker preemption/disconnect, and speculative backup execution for tail stragglers.
- **Key Deliverables:**
  - Implement `PhiAccrualDetector` tracking inter-arrival times of worker `TaskHeartbeat` messages (containing progress %, current checkpoint BLAKE3 digest, and thermal/battery state) to compute suspicion level $\phi(t)$ Adaptive to LAN vs WAN jitter.
  - Implement `CheckpointManager` enabling long-running tasks to snapshot intermediate state (`CheckpointBlob` + step index) locally and replicate lightweight checkpoint metadata to the Master/peer storage.
  - Implement automatic failover and speculative execution (`SpeculativeRetryEngine`): when a worker disconnects ($\phi > 8.0$) or emits `NodeGuardEvent::EvacuateNode`, reschedule the task on the next-best healthy peer starting from the latest verified `CheckpointBlob` rather than step 0.

### 3.3 Subphase `S4.P3.SP3`: End-to-End Distributed Job Execution Verification
- **Prompt File:** [`S4_P3_SP3.md`](S4_P3_SP3.md)
- **Scope:** Integrate `umcf-identity`, `umcf-enrollment`, `umcf-registry`, `umcf-net`, `umcf-hwprobe`, `umcf-guard`, `umcf-scheduler`, and `umcf-worker` into a complete multi-node distributed compute cluster test suite and certify Stage 4.
- **Key Deliverables:**
  - Implement `ClusterNodeRuntime` in `crates/umcf-worker/src/runtime.rs` wiring discovery, enrollment, registry sync, hardware probing, resource guard, DAG scheduler (when in `Master`/`Hybrid` role), and sandboxed worker execution into a single cohesive node runtime.
  - Implement an end-to-end multi-stage DAG benchmark job (distributed parallel data compression + cryptographic hashing + matrix reduction) executing across a 4-node heterogeneous virtual cluster (`1 Master/Hybrid Desktop`, `2 Worker Laptops`, `1 Mobile Worker`).
  - Implement `scripts/verify-stage-04.sh` validating all 9 Stage 4 subphases (`S4.P1.SP1` .. `S4.P3.SP3`), running chaos fault injection (mid-job mobile thermal throttle + worker kill + user input preemption), checking zero placeholders, and verifying Podman disk footprint `< 29.0 GiB`.

---

## 4. Acceptance Criteria (Phase-Level Measurable Gates)

- [ ] All acceptance criteria in `S4_P3_SP1.md`, `S4_P3_SP2.md`, and `S4_P3_SP3.md` are satisfied.
- [ ] Cross-subphase integration between `S4.P3.SP1`, `S4.P3.SP2`, and `S4.P3.SP3` compiles cleanly with zero warnings (`cargo clippy --workspace --all-targets -- -D warnings`).
- [ ] Line coverage across crates modified in `S4.P3` is `>= 85.0%`.
- [ ] Zero `todo!()`, `unimplemented!()`, or placeholder stubs exist in the codebase (`scripts/check-no-placeholders.sh` exits `0`).
- [ ] Podman storage utilization remains strictly `< 29.0 GiB`.

---

## 5. Required Tests (Classified Rollup)

- **Unit Tests:** All unit test suites across `S4.P3.SP1`, `S4.P3.SP2`, and `S4.P3.SP3` pass via `cargo nextest run`.
- **Integration Tests:** End-to-end phase integration tests verifying `Isolated Worker Execution Sandbox & Artifact Staging` + `Heartbeat Failure Detection, Checkpointing & Speculative Retry` + `End-to-End Distributed Job Execution Verification` interoperation.
- **Simulation Tests:** Fault-injection, corrupted input, and concurrency stress tests defined in `S4.P3.SP1..SP3`.
- **Hardware-Required Tests:** Executed on target hardware or recorded in hardware execution manifests where containerized hardware passthrough is unavailable.
- **Manually Verified Tests:** Developer visual/CLI inspection checklists completed for `S4.P3.SP1..SP3`.

---

## 6. Required Evidence & Artifacts

- `artifacts/evidence/stage-04/S4_P3_SP1_evidence.json`
- `artifacts/evidence/stage-04/S4_P3_SP2_evidence.json`
- `artifacts/evidence/stage-04/S4_P3_SP3_evidence.json`
- `artifacts/evidence/stage-04/S4_P3_PHASE_SUMMARY.json` aggregating all 3 subphase evidence bundles and phase-level coverage metrics.

---

## 7. Documentation, `TODO.md` & `CHANGELOG.md` Updates

- Verify `S4.P3.SP1`, `S4.P3.SP2`, and `S4.P3.SP3` are all checked `[x]` in `TODO.md`.
- Consolidate `S4.P3` capabilities into `CHANGELOG.md` and module-level architecture docs.

---

## 8. Rollback Criteria & Completion Gate

- **Rollback Criteria:** If phase integration tests fail or Podman disk quota exceeds `29.0 GiB` after cache pruning, roll back to the start of `S4.P3` (`v0.4.2-certified`) and isolate the failing subphase.
- **Completion Gate:** Tag repository checkpoint `v0.4.3-certified` once `S4_P3_PHASE_SUMMARY.json` reports `"status": "PASS"`.
