# Executable Subphase Prompt — `S2.P3.SP2`: Cloudflare Workers/Durable Objects Control Plane Sync

> **Stage:** Stage 2 — Rust Core, Identity, Enrollment, Device Registry and Cluster Management (`STAGE_02_PROMPT.md`)  
> **Phase:** Phase `S2.P3` — Distributed Device Registry & Cluster Coordination (`S2_P3_PROMPT.md`)  
> **Subphase:** `S2.P3.SP2` — Cloudflare Workers/Durable Objects Control Plane Sync  
> **Execution Mode:** Single-Subphase Atomic Implementation inside Rootless Podman (`<= 30 GB` Hard Disk Ceiling)

---

## 1. Role & Context

You are a Principal Distributed Systems & Cross-Platform Rust/Flutter/Edge Architect executing Subphase **`S2.P3.SP2` (Cloudflare Workers/Durable Objects Control Plane Sync)** of the Universal Distributed Mobile-First Compute Fabric (**UMCF**).

- **Subphase Scope:** Implement the primary global edge coordination service in `services/cf_control_plane` using Cloudflare Workers and Durable Objects (`ClusterCoordinatorDO`) with WebSocket hibernation and signed delta synchronization.
- **Architectural Mandate:** Implement production-grade, memory-safe, deterministic code with zero `todo!()`, `unimplemented!()`, or mock-only placeholders in production crates/services. All build, lint, and test executions must run inside rootless Podman containers while enforcing the `<= 30 GB` disk ceiling via `scripts/podman-quota-check.sh`.

---

## 2. Prerequisites & Dependencies

- **Upstream Gate:** `S2.P3.SP1` must be marked `[x]` in `TODO.md` with verified evidence JSON on disk.
- `S2.P3.SP1` completed
- **Podman Quota Pre-Check:** Run `scripts/podman-quota-check.sh` before starting implementation; total footprint must be `< 29.0 GiB` (`31,138,512,896` bytes).

---

## 3. Implementation Tasks

Execute the following concrete implementation tasks in order:

1. Implement `services/cf_control_plane/src/durable_object.ts` (`ClusterCoordinatorDO`) storing cluster registry deltas, active rendezvous endpoints, and relay descriptors in Durable Object transactional storage + SQLite backend.
2. Implement HTTP/WebSocket endpoints in `services/cf_control_plane/src/index.ts`: `POST /v1/clusters/:id/register`, `POST /v1/clusters/:id/sync-delta`, `GET /v1/clusters/:id/peers`, and `GET /v1/clusters/:id/ws` (WebSocket push for instant peer presence and NAT signaling).
3. Verify Ed25519 request signatures (`X-UMCF-DID`, `X-UMCF-Timestamp`, `X-UMCF-Signature`) using WebCrypto `Ed25519` inside the Worker before mutating Durable Object state, and implement the Rust client `CloudflareControlPlaneClient` in `crates/umcf-registry/src/cloud_sync.rs`.

---

## 4. Acceptance Criteria (Measurable)

All criteria below must be quantitatively verified before marking `S2.P3.SP2` complete:

- [ ] Unauthenticated or invalidly signed requests to `cf_control_plane` are rejected at the edge with `401 Unauthorized` (`E2001`).
- [ ] Delta sync between `umcf-registry` Rust client and `cf_control_plane` propagates new peer presence to connected WebSocket clients in `< 50 ms` in local Miniflare/Workerd simulation.
- [ ] Zero plaintext job payloads or private AI prompts are ever sent to the control plane (strictly cluster metadata, public keys, capabilities, and ICE/relay endpoints).
- [ ] Zero unresolved placeholders (`scripts/check-no-placeholders.sh` exits `0`).
- [ ] Rootless Podman disk usage after build & test remains `< 29.0 GiB` (`scripts/podman-quota-check.sh` exits `0`).

---

## 5. Required Tests (Classified)

Every test category below must be implemented, executed, and logged in the subphase evidence bundle:

- **Unit Tests:**
  - Test Worker Ed25519 signature verifier, HLC delta comparator, and Durable Object state reducer via Vitest + Miniflare.
- **Integration Tests:**
  - Run local `workerd` / Miniflare server and connect two Rust `CloudflareControlPlaneClient` nodes to verify live delta sync and WebSocket peer discovery.
- **Simulation / Fault-Injection Tests:**
  - Simulate expired timestamp (`now - 120s`) and tampered payload against Worker endpoint to verify replay protection.
- **Hardware-Required Tests:**
  - Not Required.
- **Manually Verified Tests:**
  - Review `services/cf_control_plane/wrangler.toml` bindings and route definitions.

---

## 6. Required Evidence & Artifacts

Generate and archive the following deterministic artifacts before completing `S2.P3.SP2`:

- `artifacts/evidence/stage-02/S2_P3_SP2_evidence.json`
- `artifacts/evidence/stage-02/cf_worker_vitest_report.json`
- Standard Evidence Schema (`artifacts/evidence/stage-02/S2_P3_SP2_evidence.json`):
  ```json
  {
    "subphase": "S2.P3.SP2",
    "title": "Cloudflare Workers/Durable Objects Control Plane Sync",
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

1. **Module & Rustdoc Documentation:** Document all public structs, traits, functions, error codes, and wire schemas added in `S2.P3.SP2`.
2. **`TODO.md` Update:** Flip `- [ ] S2.P3.SP2` to `- [x] S2.P3.SP2` in `TODO.md` and record the evidence file path.
3. **`CHANGELOG.md` Update:** Append a bullet under `[Unreleased] -> ### Stage 2 — Phase 3` summarizing the concrete deliverables and metrics of `S2.P3.SP2`.

---

## 8. Rollback Criteria & Completion Gate

### Rollback Criteria
Immediately revert `S2.P3.SP2` changes (`git reset --hard` to the `S2.P3.SP1` checkpoint tag) and prune container caches if any of the following occur:
1. `scripts/podman-quota-check.sh` exceeds `29.0 GiB` and automated `cargo sweep` / `podman system prune` cannot recover below `25.0 GiB`.
2. Any prior stage or subphase test suite regresses.
3. Wire protocol backward compatibility in `umcf-proto` is broken without an explicit major version negotiation path.

### Completion Gate
`S2.P3.SP2` is **COMPLETE** if and only if:
- All implementation tasks are committed with zero placeholders;
- Unit, Integration, and Simulation tests pass 100% inside Podman;
- `artifacts/evidence/stage-02/S2_P3_SP2_evidence.json` is written and validated;
- `TODO.md` and `CHANGELOG.md` are updated.
