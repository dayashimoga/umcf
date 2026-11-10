# Executable Subphase Prompt — `S2.P3.SP1`: Local Autonomous Coordinator & SQLite/Redb State Store

> **Stage:** Stage 2 — Rust Core, Identity, Enrollment, Device Registry and Cluster Management (`STAGE_02_PROMPT.md`)  
> **Phase:** Phase `S2.P3` — Distributed Device Registry & Cluster Coordination (`S2_P3_PROMPT.md`)  
> **Subphase:** `S2.P3.SP1` — Local Autonomous Coordinator & SQLite/Redb State Store  
> **Execution Mode:** Single-Subphase Atomic Implementation inside Rootless Podman (`<= 30 GB` Hard Disk Ceiling)

---

## 1. Role & Context

You are a Principal Distributed Systems & Cross-Platform Rust/Flutter/Edge Architect executing Subphase **`S2.P3.SP1` (Local Autonomous Coordinator & SQLite/Redb State Store)** of the Universal Distributed Mobile-First Compute Fabric (**UMCF**).

- **Subphase Scope:** Implement `crates/umcf-registry` with embedded ACID key-value storage (`redb`), Hybrid Logical Clocks (`HlcTimestamp`), and Observed-Remove Set / LWW-Element-Set CRDTs for node membership, capabilities, and consent records.
- **Architectural Mandate:** Implement production-grade, memory-safe, deterministic code with zero `todo!()`, `unimplemented!()`, or mock-only placeholders in production crates/services. All build, lint, and test executions must run inside rootless Podman containers while enforcing the `<= 30 GB` disk ceiling via `scripts/podman-quota-check.sh`.

---

## 2. Prerequisites & Dependencies

- **Upstream Gate:** `S2.P2.SP3` must be marked `[x]` in `TODO.md` with verified evidence JSON on disk.
- `S2.P2.SP3` completed
- **Podman Quota Pre-Check:** Run `scripts/podman-quota-check.sh` before starting implementation; total footprint must be `< 29.0 GiB` (`31,138,512,896` bytes).

---

## 3. Implementation Tasks

Execute the following concrete implementation tasks in order:

1. Create `crates/umcf-registry/src/store.rs` using `redb` with strongly-typed tables: `NODES_TABLE`, `CAPABILITIES_TABLE`, `CONSENT_TABLE`, `CERT_CRL_TABLE`, and `CLUSTER_META_TABLE`.
2. Implement `crates/umcf-registry/src/hlc.rs` providing monotonic `HlcTimestamp { wall_ms: u64, logical: u32, node_hash: u32 }` with clock-drift bounds enforcement (`max_drift_ms = 5_000`).
3. Implement `crates/umcf-registry/src/crdt.rs` defining `ClusterRegistryState` with delta-state CRDT merge (`merge_delta(&mut self, remote_delta: RegistryDelta) -> MergeSummary`) that is commutative, associative, and idempotent.

---

## 4. Acceptance Criteria (Measurable)

All criteria below must be quantitatively verified before marking `S2.P3.SP1` complete:

- [ ] `redb` transactions persist node registration, capability updates, and consent updates in `< 1.5 ms` p99 on SSD/flash storage.
- [ ] `ClusterRegistryState::merge_delta` satisfies mathematical CRDT laws (`A.merge(B) == B.merge(A)` and `A.merge(A) == A`) across 10,000 randomized `proptest` operation sequences.
- [ ] Local coordinator operates with 100% functionality when WAN/internet is completely unreachable.
- [ ] Zero unresolved placeholders (`scripts/check-no-placeholders.sh` exits `0`).
- [ ] Rootless Podman disk usage after build & test remains `< 29.0 GiB` (`scripts/podman-quota-check.sh` exits `0`).

---

## 5. Required Tests (Classified)

Every test category below must be implemented, executed, and logged in the subphase evidence bundle:

- **Unit Tests:**
  - Test `HlcTimestamp` monotonicity, drift rejection, and `redb` CRUD operations.
- **Integration Tests:**
  - Create 3 independent `LocalRegistryStore` instances, apply concurrent node joins, capability mutations, and revocations, exchange deltas in all 6 permutation orders, and assert byte-identical final state.
- **Simulation / Fault-Injection Tests:**
  - Simulate crash during `redb` write and verify ACID recovery on restart with zero corrupted entries.
- **Hardware-Required Tests:**
  - Not Required.
- **Manually Verified Tests:**
  - Inspect `redb` database file size growth and compaction metrics.

---

## 6. Required Evidence & Artifacts

Generate and archive the following deterministic artifacts before completing `S2.P3.SP1`:

- `artifacts/evidence/stage-02/S2_P3_SP1_evidence.json`
- `artifacts/evidence/stage-02/crdt_proptest_summary.json`
- Standard Evidence Schema (`artifacts/evidence/stage-02/S2_P3_SP1_evidence.json`):
  ```json
  {
    "subphase": "S2.P3.SP1",
    "title": "Local Autonomous Coordinator & SQLite/Redb State Store",
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

1. **Module & Rustdoc Documentation:** Document all public structs, traits, functions, error codes, and wire schemas added in `S2.P3.SP1`.
2. **`TODO.md` Update:** Flip `- [ ] S2.P3.SP1` to `- [x] S2.P3.SP1` in `TODO.md` and record the evidence file path.
3. **`CHANGELOG.md` Update:** Append a bullet under `[Unreleased] -> ### Stage 2 — Phase 3` summarizing the concrete deliverables and metrics of `S2.P3.SP1`.

---

## 8. Rollback Criteria & Completion Gate

### Rollback Criteria
Immediately revert `S2.P3.SP1` changes (`git reset --hard` to the `S2.P2.SP3` checkpoint tag) and prune container caches if any of the following occur:
1. `scripts/podman-quota-check.sh` exceeds `29.0 GiB` and automated `cargo sweep` / `podman system prune` cannot recover below `25.0 GiB`.
2. Any prior stage or subphase test suite regresses.
3. Wire protocol backward compatibility in `umcf-proto` is broken without an explicit major version negotiation path.

### Completion Gate
`S2.P3.SP1` is **COMPLETE** if and only if:
- All implementation tasks are committed with zero placeholders;
- Unit, Integration, and Simulation tests pass 100% inside Podman;
- `artifacts/evidence/stage-02/S2_P3_SP1_evidence.json` is written and validated;
- `TODO.md` and `CHANGELOG.md` are updated.
