# Executable Subphase Prompt — `S2.P3.SP3`: Google Cloud Run Fallback Control Plane & Split-Brain Reconciliation

> **Stage:** Stage 2 — Rust Core, Identity, Enrollment, Device Registry and Cluster Management (`STAGE_02_PROMPT.md`)  
> **Phase:** Phase `S2.P3` — Distributed Device Registry & Cluster Coordination (`S2_P3_PROMPT.md`)  
> **Subphase:** `S2.P3.SP3` — Google Cloud Run Fallback Control Plane & Split-Brain Reconciliation  
> **Execution Mode:** Single-Subphase Atomic Implementation inside Rootless Podman (`<= 30 GB` Hard Disk Ceiling)

---

## 1. Role & Context

You are a Principal Distributed Systems & Cross-Platform Rust/Flutter/Edge Architect executing Subphase **`S2.P3.SP3` (Google Cloud Run Fallback Control Plane & Split-Brain Reconciliation)** of the Universal Distributed Mobile-First Compute Fabric (**UMCF**).

- **Subphase Scope:** Implement the stateless Rust Axum fallback control plane in `services/cloudrun_fallback` (compatible with Google Cloud Run), automatic multi-endpoint failover in `umcf-registry`, split-brain partition reconciliation, and Stage 2 Certification.
- **Architectural Mandate:** Implement production-grade, memory-safe, deterministic code with zero `todo!()`, `unimplemented!()`, or mock-only placeholders in production crates/services. All build, lint, and test executions must run inside rootless Podman containers while enforcing the `<= 30 GB` disk ceiling via `scripts/podman-quota-check.sh`.

---

## 2. Prerequisites & Dependencies

- **Upstream Gate:** `S2.P3.SP2` must be marked `[x]` in `TODO.md` with verified evidence JSON on disk.
- `S2.P3.SP2` completed
- **Podman Quota Pre-Check:** Run `scripts/podman-quota-check.sh` before starting implementation; total footprint must be `< 29.0 GiB` (`31,138,512,896` bytes).

---

## 3. Implementation Tasks

Execute the following concrete implementation tasks in order:

1. Implement `services/cloudrun_fallback/src/main.rs` using Axum + Tokio exposing the exact same `/v1/clusters/:id/*` REST and WebSocket wire contract as `cf_control_plane`, with pluggable state persistence and container health check `GET /healthz`.
2. Implement `FailoverControlPlaneClient` in `crates/umcf-registry/src/failover.rs` supporting circuit-breaker health probing: Primary (`Cloudflare Workers`) -> Secondary (`Google Cloud Run`) -> Tertiary (`LAN Autonomous Master-Only Mode`).
3. Implement split-brain partition healer (`reconcile_partition`) that merges diverged LAN sub-clusters when WAN connectivity resumes, resolves conflicting Master terms by demoting the lower-term/lower-fitness Master to Worker/Hybrid, and create `scripts/verify-stage-02.sh`.

---

## 4. Acceptance Criteria (Measurable)

All criteria below must be quantitatively verified before marking `S2.P3.SP3` complete:

- [ ] When the primary Cloudflare endpoint returns HTTP 5xx or times out (`> 1500 ms`), `FailoverControlPlaneClient` switches to the Cloud Run fallback endpoint within `< 2.0 seconds` without losing queued registry deltas.
- [ ] When both cloud endpoints are unreachable, the cluster continues operating autonomously on LAN; upon cloud restoration, split-brain sub-clusters reconcile to an identical CRDT digest within `< 1.0 second`.
- [ ] `scripts/verify-stage-02.sh` exits `0`, certifying all 9 Stage 2 subphases and confirming Podman disk usage `< 29.0 GiB`.
- [ ] Zero unresolved placeholders (`scripts/check-no-placeholders.sh` exits `0`).
- [ ] Rootless Podman disk usage after build & test remains `< 29.0 GiB` (`scripts/podman-quota-check.sh` exits `0`).

---

## 5. Required Tests (Classified)

Every test category below must be implemented, executed, and logged in the subphase evidence bundle:

- **Unit Tests:**
  - Test circuit-breaker state transitions (`Closed -> Open -> HalfOpen -> Closed`) and split-brain Master term conflict resolution.
- **Integration Tests:**
  - Start both `cf_control_plane` (mock/workerd) and `cloudrun_fallback` (Axum), kill primary mid-sync, verify seamless failover to `cloudrun_fallback`, then kill both and verify LAN autonomous continuity.
- **Simulation / Fault-Injection Tests:**
  - Partition a 6-node cluster into two 3-node islands (`Island A` and `Island B`), elect separate Masters in each island, mutate capabilities/nodes independently, heal partition, and assert single Master + identical 6-node registry state.
- **Hardware-Required Tests:**
  - Not Required.
- **Manually Verified Tests:**
  - Inspect `STAGE_02_CERTIFICATION.json` and verify `TODO.md` and `CHANGELOG.md` updates.

---

## 6. Required Evidence & Artifacts

Generate and archive the following deterministic artifacts before completing `S2.P3.SP3`:

- `artifacts/evidence/stage-02/S2_P3_SP3_evidence.json`
- `artifacts/evidence/stage-02/split_brain_reconciliation_trace.json`
- `artifacts/evidence/stage-02/STAGE_02_CERTIFICATION.json`
- Standard Evidence Schema (`artifacts/evidence/stage-02/S2_P3_SP3_evidence.json`):
  ```json
  {
    "subphase": "S2.P3.SP3",
    "title": "Google Cloud Run Fallback Control Plane & Split-Brain Reconciliation",
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

1. **Module & Rustdoc Documentation:** Document all public structs, traits, functions, error codes, and wire schemas added in `S2.P3.SP3`.
2. **`TODO.md` Update:** Flip `- [ ] S2.P3.SP3` to `- [x] S2.P3.SP3` in `TODO.md` and record the evidence file path.
3. **`CHANGELOG.md` Update:** Append a bullet under `[Unreleased] -> ### Stage 2 — Phase 3` summarizing the concrete deliverables and metrics of `S2.P3.SP3`.

---

## 8. Rollback Criteria & Completion Gate

### Rollback Criteria
Immediately revert `S2.P3.SP3` changes (`git reset --hard` to the `S2.P3.SP2` checkpoint tag) and prune container caches if any of the following occur:
1. `scripts/podman-quota-check.sh` exceeds `29.0 GiB` and automated `cargo sweep` / `podman system prune` cannot recover below `25.0 GiB`.
2. Any prior stage or subphase test suite regresses.
3. Wire protocol backward compatibility in `umcf-proto` is broken without an explicit major version negotiation path.

### Completion Gate
`S2.P3.SP3` is **COMPLETE** if and only if:
- All implementation tasks are committed with zero placeholders;
- Unit, Integration, and Simulation tests pass 100% inside Podman;
- `artifacts/evidence/stage-02/S2_P3_SP3_evidence.json` is written and validated;
- `TODO.md` and `CHANGELOG.md` are updated.
