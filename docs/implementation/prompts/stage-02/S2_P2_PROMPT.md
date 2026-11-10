# Executable Phase Prompt — `S2.P2`: QR Enrollment, RBAC & Dynamic Master/Worker Role Engine

> **Stage:** Stage 2 — Rust Core, Identity, Enrollment, Device Registry and Cluster Management (`STAGE_02_PROMPT.md`)  
> **Phase:** `S2.P2` — QR Enrollment, RBAC & Dynamic Master/Worker Role Engine  
> **Subphases:** `S2.P2.SP1`, `S2.P2.SP2`, `S2.P2.SP3`  
> **Target Environment:** Rootless Podman (`<= 30 GB` Hard Disk Ceiling)

---

## 1. Role & Context

You are the Lead Systems Architect executing Phase **`S2.P2` (QR Enrollment, RBAC & Dynamic Master/Worker Role Engine)** of Stage 2 in the Universal Distributed Mobile-First Compute Fabric (**UMCF**).

- **Phase Objective:** Implement frictionless short-lived QR code and URI/PIN enrollment (`umcf-enrollment`), capability-scoped RBAC with explicit resource-sharing consent policies, and the dynamic Master/Worker/Hybrid election state machine.
- **Execution Discipline:** Execute `S2.P2.SP1`, `S2.P2.SP2`, and `S2.P2.SP3` sequentially. Do not begin a subsequent subphase until the current subphase's Completion Gate and Evidence JSON are verified.

---

## 2. Prerequisites & Dependencies

- **Upstream Dependency:** `S2.P1` must be 100% complete and certified.
- **Target Crates / Services:** `crates/umcf-identity, crates/umcf-enrollment, crates/umcf-registry, services/cf_control_plane, services/cloudrun_fallback`
- **Quota Enforcement:** `scripts/podman-quota-check.sh` must pass before and after each subphase (`< 29.0 GiB`).

---

## 3. Implementation Tasks & Subphase Execution Sequence

Execute the three constituent subphases in strict sequential order:

### 3.1 Subphase `S2.P2.SP1`: Short-Lived QR Enrollment Token Generator & Scanner Flow
- **Prompt File:** [`S2_P2_SP1.md`](S2_P2_SP1.md)
- **Scope:** Implement compact, cryptographically signed, short-lived enrollment tokens (`umcf://enroll?...`), SVG/PNG/terminal-UTF8 QR code rendering, and one-time SPAKE2+/X25519 enrollment exchange in `crates/umcf-enrollment`.
- **Key Deliverables:**
  - Create `crates/umcf-enrollment/src/token.rs` defining `EnrollmentToken` containing `cluster_id`, `issuer_did`, `ephemeral_x25519_pk`, `bootstrap_endpoints: Vec<SocketAddr>`, `one_time_nonce: [u8; 16]`, `expires_at_ms` (default TTL `180s`, max `600s`), `proposed_role`, and `issuer_ed25519_sig`.
  - Implement `EnrollmentToken::to_uri()` (`umcf://enroll/v1/<base64url>`) and QR matrix generator (`to_svg()`, `to_unicode_blocks()`) in `crates/umcf-enrollment/src/qr.rs`.
  - Implement `EnrollmentHandshake` in `crates/umcf-enrollment/src/handshake.rs`: candidate scans token, verifies signature & TTL, establishes X25519 + HKDF-SHA256 encrypted channel, submits `JoinRequest` (with `DidDocument` + `HardwareCapabilityDescriptor`), and receives signed `NodeCertificate`.

### 3.2 Subphase `S2.P2.SP2`: Fine-Grained RBAC & Resource-Sharing Consent Engine
- **Prompt File:** [`S2_P2_SP2.md`](S2_P2_SP2.md)
- **Scope:** Implement capability-based Role-Based Access Control (`RbacPolicyEngine`) and user-controlled Resource-Sharing Consent manifests (`ResourceConsentPolicy`) governing what CPU, GPU, NPU, RAM, storage, and workloads a peer is allowed to consume.
- **Key Deliverables:**
  - Implement `crates/umcf-enrollment/src/rbac.rs` defining `Role` (`ClusterOwner`, `MasterCoordinator`, `HybridNode`, `ComputeWorker`, `StorageProvider`, `ObserverGuest`), `Permission` (`SubmitJob`, `ScheduleTask`, `ReadClusterState`, `EnrollNode`, `RevokeNode`, `AccessPrivateAi`, `StreamDesktop`, `ReadWriteStorage`), and `RbacPolicyEngine::authorize(actor_did, permission, resource_scope)`.
  - Implement `crates/umcf-enrollment/src/consent.rs` defining `ResourceConsentPolicy` signed by the device owner: `max_cpu_percent: u8`, `max_ram_bytes: ByteSize`, `max_vram_bytes: ByteSize`, `max_storage_bytes: ByteSize`, `allowed_workload_kinds: BitFlags<WorkloadKind>`, `require_charging: bool`, `min_battery_percent: u8`, `quiet_hours_schedule`, and `allowed_requester_dids: AllowList`.
  - Ensure local `ConsentEnforcer` intercepts every incoming task assignment before execution and verifies both cluster RBAC and local device owner `ResourceConsentPolicy`.

### 3.3 Subphase `S2.P2.SP3`: Dynamic Master/Worker/Hybrid State Machine & Election
- **Prompt File:** [`S2_P2_SP3.md`](S2_P2_SP3.md)
- **Scope:** Implement the deterministic node role state machine (`Master`, `Worker`, `Hybrid`, `AutonomousSolo`) and lightweight term-based leader election with fitness scoring (uptime, power state, RAM/CPU headroom, network stability).
- **Key Deliverables:**
  - Implement `crates/umcf-enrollment/src/role_engine.rs` defining `NodeRoleState` (`Master`, `Worker`, `Hybrid`, `Candidate`, `AutonomousSolo`) and transition rules allowing any node to act as Master, Worker, or both (`Hybrid`).
  - Implement `MasterFitnessScore` calculation in `crates/umcf-enrollment/src/election.rs`: weights AC/charging power (`+400`), non-mobile or docked stability (`+250`), available RAM (`+150`), low packet loss (`+100`), and deterministic tie-breaking via BLAKE3(`term || did`).
  - Implement Raft-inspired term-based leader election (`RequestVote`, `VoteGranted`, `LeaderHeartbeat`) with configurable election timeout (`1500 ms` – `3000 ms` jittered) and graceful leadership step-down (`LeaderTransfer`) when a Master unplugs from AC power or drops below battery threshold.

---

## 4. Acceptance Criteria (Phase-Level Measurable Gates)

- [ ] All acceptance criteria in `S2_P2_SP1.md`, `S2_P2_SP2.md`, and `S2_P2_SP3.md` are satisfied.
- [ ] Cross-subphase integration between `S2.P2.SP1`, `S2.P2.SP2`, and `S2.P2.SP3` compiles cleanly with zero warnings (`cargo clippy --workspace --all-targets -- -D warnings`).
- [ ] Line coverage across crates modified in `S2.P2` is `>= 85.0%`.
- [ ] Zero `todo!()`, `unimplemented!()`, or placeholder stubs exist in the codebase (`scripts/check-no-placeholders.sh` exits `0`).
- [ ] Podman storage utilization remains strictly `< 29.0 GiB`.

---

## 5. Required Tests (Classified Rollup)

- **Unit Tests:** All unit test suites across `S2.P2.SP1`, `S2.P2.SP2`, and `S2.P2.SP3` pass via `cargo nextest run`.
- **Integration Tests:** End-to-end phase integration tests verifying `Short-Lived QR Enrollment Token Generator & Scanner Flow` + `Fine-Grained RBAC & Resource-Sharing Consent Engine` + `Dynamic Master/Worker/Hybrid State Machine & Election` interoperation.
- **Simulation Tests:** Fault-injection, corrupted input, and concurrency stress tests defined in `S2.P2.SP1..SP3`.
- **Hardware-Required Tests:** Executed on target hardware or recorded in hardware execution manifests where containerized hardware passthrough is unavailable.
- **Manually Verified Tests:** Developer visual/CLI inspection checklists completed for `S2.P2.SP1..SP3`.

---

## 6. Required Evidence & Artifacts

- `artifacts/evidence/stage-02/S2_P2_SP1_evidence.json`
- `artifacts/evidence/stage-02/S2_P2_SP2_evidence.json`
- `artifacts/evidence/stage-02/S2_P2_SP3_evidence.json`
- `artifacts/evidence/stage-02/S2_P2_PHASE_SUMMARY.json` aggregating all 3 subphase evidence bundles and phase-level coverage metrics.

---

## 7. Documentation, `TODO.md` & `CHANGELOG.md` Updates

- Verify `S2.P2.SP1`, `S2.P2.SP2`, and `S2.P2.SP3` are all checked `[x]` in `TODO.md`.
- Consolidate `S2.P2` capabilities into `CHANGELOG.md` and module-level architecture docs.

---

## 8. Rollback Criteria & Completion Gate

- **Rollback Criteria:** If phase integration tests fail or Podman disk quota exceeds `29.0 GiB` after cache pruning, roll back to the start of `S2.P2` (`v0.2.1-certified`) and isolate the failing subphase.
- **Completion Gate:** Tag repository checkpoint `v0.2.2-certified` once `S2_P2_PHASE_SUMMARY.json` reports `"status": "PASS"`.
