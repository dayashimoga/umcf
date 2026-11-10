# Executable Phase Prompt — `S2.P3`: Distributed Device Registry & Cluster Coordination

> **Stage:** Stage 2 — Rust Core, Identity, Enrollment, Device Registry and Cluster Management (`STAGE_02_PROMPT.md`)  
> **Phase:** `S2.P3` — Distributed Device Registry & Cluster Coordination  
> **Subphases:** `S2.P3.SP1`, `S2.P3.SP2`, `S2.P3.SP3`  
> **Target Environment:** Rootless Podman (`<= 30 GB` Hard Disk Ceiling)

---

## 1. Role & Context

You are the Lead Systems Architect executing Phase **`S2.P3` (Distributed Device Registry & Cluster Coordination)** of Stage 2 in the Universal Distributed Mobile-First Compute Fabric (**UMCF**).

- **Phase Objective:** Implement the persistent local autonomous registry (`umcf-registry` backed by pure-Rust `redb`), the primary edge control plane (`services/cf_control_plane` on Cloudflare Workers + Durable Objects), and the stateless container fallback (`services/cloudrun_fallback` on Google Cloud Run) with HLC CRDT split-brain reconciliation.
- **Execution Discipline:** Execute `S2.P3.SP1`, `S2.P3.SP2`, and `S2.P3.SP3` sequentially. Do not begin a subsequent subphase until the current subphase's Completion Gate and Evidence JSON are verified.

---

## 2. Prerequisites & Dependencies

- **Upstream Dependency:** `S2.P2` must be 100% complete and certified.
- **Target Crates / Services:** `crates/umcf-identity, crates/umcf-enrollment, crates/umcf-registry, services/cf_control_plane, services/cloudrun_fallback`
- **Quota Enforcement:** `scripts/podman-quota-check.sh` must pass before and after each subphase (`< 29.0 GiB`).

---

## 3. Implementation Tasks & Subphase Execution Sequence

Execute the three constituent subphases in strict sequential order:

### 3.1 Subphase `S2.P3.SP1`: Local Autonomous Coordinator & SQLite/Redb State Store
- **Prompt File:** [`S2_P3_SP1.md`](S2_P3_SP1.md)
- **Scope:** Implement `crates/umcf-registry` with embedded ACID key-value storage (`redb`), Hybrid Logical Clocks (`HlcTimestamp`), and Observed-Remove Set / LWW-Element-Set CRDTs for node membership, capabilities, and consent records.
- **Key Deliverables:**
  - Create `crates/umcf-registry/src/store.rs` using `redb` with strongly-typed tables: `NODES_TABLE`, `CAPABILITIES_TABLE`, `CONSENT_TABLE`, `CERT_CRL_TABLE`, and `CLUSTER_META_TABLE`.
  - Implement `crates/umcf-registry/src/hlc.rs` providing monotonic `HlcTimestamp { wall_ms: u64, logical: u32, node_hash: u32 }` with clock-drift bounds enforcement (`max_drift_ms = 5_000`).
  - Implement `crates/umcf-registry/src/crdt.rs` defining `ClusterRegistryState` with delta-state CRDT merge (`merge_delta(&mut self, remote_delta: RegistryDelta) -> MergeSummary`) that is commutative, associative, and idempotent.

### 3.2 Subphase `S2.P3.SP2`: Cloudflare Workers/Durable Objects Control Plane Sync
- **Prompt File:** [`S2_P3_SP2.md`](S2_P3_SP2.md)
- **Scope:** Implement the primary global edge coordination service in `services/cf_control_plane` using Cloudflare Workers and Durable Objects (`ClusterCoordinatorDO`) with WebSocket hibernation and signed delta synchronization.
- **Key Deliverables:**
  - Implement `services/cf_control_plane/src/durable_object.ts` (`ClusterCoordinatorDO`) storing cluster registry deltas, active rendezvous endpoints, and relay descriptors in Durable Object transactional storage + SQLite backend.
  - Implement HTTP/WebSocket endpoints in `services/cf_control_plane/src/index.ts`: `POST /v1/clusters/:id/register`, `POST /v1/clusters/:id/sync-delta`, `GET /v1/clusters/:id/peers`, and `GET /v1/clusters/:id/ws` (WebSocket push for instant peer presence and NAT signaling).
  - Verify Ed25519 request signatures (`X-UMCF-DID`, `X-UMCF-Timestamp`, `X-UMCF-Signature`) using WebCrypto `Ed25519` inside the Worker before mutating Durable Object state, and implement the Rust client `CloudflareControlPlaneClient` in `crates/umcf-registry/src/cloud_sync.rs`.

### 3.3 Subphase `S2.P3.SP3`: Google Cloud Run Fallback Control Plane & Split-Brain Reconciliation
- **Prompt File:** [`S2_P3_SP3.md`](S2_P3_SP3.md)
- **Scope:** Implement the stateless Rust Axum fallback control plane in `services/cloudrun_fallback` (compatible with Google Cloud Run), automatic multi-endpoint failover in `umcf-registry`, split-brain partition reconciliation, and Stage 2 Certification.
- **Key Deliverables:**
  - Implement `services/cloudrun_fallback/src/main.rs` using Axum + Tokio exposing the exact same `/v1/clusters/:id/*` REST and WebSocket wire contract as `cf_control_plane`, with pluggable state persistence and container health check `GET /healthz`.
  - Implement `FailoverControlPlaneClient` in `crates/umcf-registry/src/failover.rs` supporting circuit-breaker health probing: Primary (`Cloudflare Workers`) -> Secondary (`Google Cloud Run`) -> Tertiary (`LAN Autonomous Master-Only Mode`).
  - Implement split-brain partition healer (`reconcile_partition`) that merges diverged LAN sub-clusters when WAN connectivity resumes, resolves conflicting Master terms by demoting the lower-term/lower-fitness Master to Worker/Hybrid, and create `scripts/verify-stage-02.sh`.

---

## 4. Acceptance Criteria (Phase-Level Measurable Gates)

- [ ] All acceptance criteria in `S2_P3_SP1.md`, `S2_P3_SP2.md`, and `S2_P3_SP3.md` are satisfied.
- [ ] Cross-subphase integration between `S2.P3.SP1`, `S2.P3.SP2`, and `S2.P3.SP3` compiles cleanly with zero warnings (`cargo clippy --workspace --all-targets -- -D warnings`).
- [ ] Line coverage across crates modified in `S2.P3` is `>= 85.0%`.
- [ ] Zero `todo!()`, `unimplemented!()`, or placeholder stubs exist in the codebase (`scripts/check-no-placeholders.sh` exits `0`).
- [ ] Podman storage utilization remains strictly `< 29.0 GiB`.

---

## 5. Required Tests (Classified Rollup)

- **Unit Tests:** All unit test suites across `S2.P3.SP1`, `S2.P3.SP2`, and `S2.P3.SP3` pass via `cargo nextest run`.
- **Integration Tests:** End-to-end phase integration tests verifying `Local Autonomous Coordinator & SQLite/Redb State Store` + `Cloudflare Workers/Durable Objects Control Plane Sync` + `Google Cloud Run Fallback Control Plane & Split-Brain Reconciliation` interoperation.
- **Simulation Tests:** Fault-injection, corrupted input, and concurrency stress tests defined in `S2.P3.SP1..SP3`.
- **Hardware-Required Tests:** Executed on target hardware or recorded in hardware execution manifests where containerized hardware passthrough is unavailable.
- **Manually Verified Tests:** Developer visual/CLI inspection checklists completed for `S2.P3.SP1..SP3`.

---

## 6. Required Evidence & Artifacts

- `artifacts/evidence/stage-02/S2_P3_SP1_evidence.json`
- `artifacts/evidence/stage-02/S2_P3_SP2_evidence.json`
- `artifacts/evidence/stage-02/S2_P3_SP3_evidence.json`
- `artifacts/evidence/stage-02/S2_P3_PHASE_SUMMARY.json` aggregating all 3 subphase evidence bundles and phase-level coverage metrics.

---

## 7. Documentation, `TODO.md` & `CHANGELOG.md` Updates

- Verify `S2.P3.SP1`, `S2.P3.SP2`, and `S2.P3.SP3` are all checked `[x]` in `TODO.md`.
- Consolidate `S2.P3` capabilities into `CHANGELOG.md` and module-level architecture docs.

---

## 8. Rollback Criteria & Completion Gate

- **Rollback Criteria:** If phase integration tests fail or Podman disk quota exceeds `29.0 GiB` after cache pruning, roll back to the start of `S2.P3` (`v0.2.2-certified`) and isolate the failing subphase.
- **Completion Gate:** Tag repository checkpoint `v0.2.3-certified` once `S2_P3_PHASE_SUMMARY.json` reports `"status": "PASS"`.
