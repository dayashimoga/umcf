# Executable Subphase Prompt — `S2.P2.SP2`: Fine-Grained RBAC & Resource-Sharing Consent Engine

> **Stage:** Stage 2 — Rust Core, Identity, Enrollment, Device Registry and Cluster Management (`STAGE_02_PROMPT.md`)  
> **Phase:** Phase `S2.P2` — QR Enrollment, RBAC & Dynamic Master/Worker Role Engine (`S2_P2_PROMPT.md`)  
> **Subphase:** `S2.P2.SP2` — Fine-Grained RBAC & Resource-Sharing Consent Engine  
> **Execution Mode:** Single-Subphase Atomic Implementation inside Rootless Podman (`<= 30 GB` Hard Disk Ceiling)

---

## 1. Role & Context

You are a Principal Distributed Systems & Cross-Platform Rust/Flutter/Edge Architect executing Subphase **`S2.P2.SP2` (Fine-Grained RBAC & Resource-Sharing Consent Engine)** of the Universal Distributed Mobile-First Compute Fabric (**UMCF**).

- **Subphase Scope:** Implement capability-based Role-Based Access Control (`RbacPolicyEngine`) and user-controlled Resource-Sharing Consent manifests (`ResourceConsentPolicy`) governing what CPU, GPU, NPU, RAM, storage, and workloads a peer is allowed to consume.
- **Architectural Mandate:** Implement production-grade, memory-safe, deterministic code with zero `todo!()`, `unimplemented!()`, or mock-only placeholders in production crates/services. All build, lint, and test executions must run inside rootless Podman containers while enforcing the `<= 30 GB` disk ceiling via `scripts/podman-quota-check.sh`.

---

## 2. Prerequisites & Dependencies

- **Upstream Gate:** `S2.P2.SP1` must be marked `[x]` in `TODO.md` with verified evidence JSON on disk.
- `S2.P2.SP1` completed
- **Podman Quota Pre-Check:** Run `scripts/podman-quota-check.sh` before starting implementation; total footprint must be `< 29.0 GiB` (`31,138,512,896` bytes).

---

## 3. Implementation Tasks

Execute the following concrete implementation tasks in order:

1. Implement `crates/umcf-enrollment/src/rbac.rs` defining `Role` (`ClusterOwner`, `MasterCoordinator`, `HybridNode`, `ComputeWorker`, `StorageProvider`, `ObserverGuest`), `Permission` (`SubmitJob`, `ScheduleTask`, `ReadClusterState`, `EnrollNode`, `RevokeNode`, `AccessPrivateAi`, `StreamDesktop`, `ReadWriteStorage`), and `RbacPolicyEngine::authorize(actor_did, permission, resource_scope)`.
2. Implement `crates/umcf-enrollment/src/consent.rs` defining `ResourceConsentPolicy` signed by the device owner: `max_cpu_percent: u8`, `max_ram_bytes: ByteSize`, `max_vram_bytes: ByteSize`, `max_storage_bytes: ByteSize`, `allowed_workload_kinds: BitFlags<WorkloadKind>`, `require_charging: bool`, `min_battery_percent: u8`, `quiet_hours_schedule`, and `allowed_requester_dids: AllowList`.
3. Ensure local `ConsentEnforcer` intercepts every incoming task assignment before execution and verifies both cluster RBAC and local device owner `ResourceConsentPolicy`.

---

## 4. Acceptance Criteria (Measurable)

All criteria below must be quantitatively verified before marking `S2.P2.SP2` complete:

- [ ] Default-deny authorization: any ungranted `(Role, Permission)` pair returns `Err(E2201_RBAC_DENIED)` in `< 5 µs`.
- [ ] No external Master node can override a Worker node's locally signed `ResourceConsentPolicy` (local sovereignty invariant).
- [ ] Changes to `ResourceConsentPolicy` take effect immediately (`< 10 ms`) and emit a signed `ConsentUpdatedEvent`.
- [ ] Zero unresolved placeholders (`scripts/check-no-placeholders.sh` exits `0`).
- [ ] Rootless Podman disk usage after build & test remains `< 29.0 GiB` (`scripts/podman-quota-check.sh` exits `0`).

---

## 5. Required Tests (Classified)

Every test category below must be implemented, executed, and logged in the subphase evidence bundle:

- **Unit Tests:**
  - Test full RBAC permission matrix across all 6 roles and 8 permissions, plus `ResourceConsentPolicy` validation rules.
- **Integration Tests:**
  - Verify a `MasterCoordinator` attempting to schedule a GPU job on a `ComputeWorker` whose `ResourceConsentPolicy` disables GPU workloads is rejected by the worker's `ConsentEnforcer`.
- **Simulation / Fault-Injection Tests:**
  - Simulate forged `ResourceConsentPolicy` signed by a foreign Master key instead of the local device owner key and verify rejection.
- **Hardware-Required Tests:**
  - Not Required.
- **Manually Verified Tests:**
  - Review serialized JSON format of `ResourceConsentPolicy` for UI binding clarity.

---

## 6. Required Evidence & Artifacts

Generate and archive the following deterministic artifacts before completing `S2.P2.SP2`:

- `artifacts/evidence/stage-02/S2_P2_SP2_evidence.json`
- `artifacts/evidence/stage-02/rbac_consent_matrix.json`
- Standard Evidence Schema (`artifacts/evidence/stage-02/S2_P2_SP2_evidence.json`):
  ```json
  {
    "subphase": "S2.P2.SP2",
    "title": "Fine-Grained RBAC & Resource-Sharing Consent Engine",
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

1. **Module & Rustdoc Documentation:** Document all public structs, traits, functions, error codes, and wire schemas added in `S2.P2.SP2`.
2. **`TODO.md` Update:** Flip `- [ ] S2.P2.SP2` to `- [x] S2.P2.SP2` in `TODO.md` and record the evidence file path.
3. **`CHANGELOG.md` Update:** Append a bullet under `[Unreleased] -> ### Stage 2 — Phase 2` summarizing the concrete deliverables and metrics of `S2.P2.SP2`.

---

## 8. Rollback Criteria & Completion Gate

### Rollback Criteria
Immediately revert `S2.P2.SP2` changes (`git reset --hard` to the `S2.P2.SP1` checkpoint tag) and prune container caches if any of the following occur:
1. `scripts/podman-quota-check.sh` exceeds `29.0 GiB` and automated `cargo sweep` / `podman system prune` cannot recover below `25.0 GiB`.
2. Any prior stage or subphase test suite regresses.
3. Wire protocol backward compatibility in `umcf-proto` is broken without an explicit major version negotiation path.

### Completion Gate
`S2.P2.SP2` is **COMPLETE** if and only if:
- All implementation tasks are committed with zero placeholders;
- Unit, Integration, and Simulation tests pass 100% inside Podman;
- `artifacts/evidence/stage-02/S2_P2_SP2_evidence.json` is written and validated;
- `TODO.md` and `CHANGELOG.md` are updated.
