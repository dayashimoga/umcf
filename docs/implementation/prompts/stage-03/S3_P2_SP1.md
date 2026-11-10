# Executable Subphase Prompt — `S3.P2.SP1`: STUN/ICE Candidate Gathering & UDP Hole Punching

> **Stage:** Stage 3 — LAN/Remote Networking, Security, Hardware Discovery and Benchmarking (`STAGE_03_PROMPT.md`)  
> **Phase:** Phase `S3.P2` — NAT Traversal, STUN/ICE Hole Punching & Relay Fallback (`S3_P2_PROMPT.md`)  
> **Subphase:** `S3.P2.SP1` — STUN/ICE Candidate Gathering & UDP Hole Punching  
> **Execution Mode:** Single-Subphase Atomic Implementation inside Rootless Podman (`<= 30 GB` Hard Disk Ceiling)

---

## 1. Role & Context

You are a Principal Distributed Systems & Cross-Platform Rust/Flutter/Edge Architect executing Subphase **`S3.P2.SP1` (STUN/ICE Candidate Gathering & UDP Hole Punching)** of the Universal Distributed Mobile-First Compute Fabric (**UMCF**).

- **Subphase Scope:** Implement RFC 8489 STUN binding requests, NAT mapping/filtering behavior classification (Endpoint-Independent, Address-Dependent, Symmetric), ICE candidate gathering (`Host`, `ServerReflexive`, `PeerReflexive`), and synchronized UDP hole punching in `crates/umcf-net/src/nat.rs`.
- **Architectural Mandate:** Implement production-grade, memory-safe, deterministic code with zero `todo!()`, `unimplemented!()`, or mock-only placeholders in production crates/services. All build, lint, and test executions must run inside rootless Podman containers while enforcing the `<= 30 GB` disk ceiling via `scripts/podman-quota-check.sh`.

---

## 2. Prerequisites & Dependencies

- **Upstream Gate:** `S3.P1.SP3` must be marked `[x]` in `TODO.md` with verified evidence JSON on disk.
- `S3.P1.SP3` completed
- **Podman Quota Pre-Check:** Run `scripts/podman-quota-check.sh` before starting implementation; total footprint must be `< 29.0 GiB` (`31,138,512,896` bytes).

---

## 3. Implementation Tasks

Execute the following concrete implementation tasks in order:

1. Implement `crates/umcf-net/src/stun.rs` encoding/decoding RFC 8489 STUN `BindingRequest` and `BindingResponse` (`XOR-MAPPED-ADDRESS`, `FINGERPRINT`, `SOFTWARE`) and querying multiple STUN endpoints concurrently.
2. Implement `NatClassifier` in `crates/umcf-net/src/nat.rs` detecting `OpenInternet`, `FullCone / EndpointIndependent`, `RestrictedCone`, `PortRestrictedCone`, or `SymmetricNat`.
3. Implement `HolePunchCoordinator` exchanging signed `IceCandidateSet` payloads over the Stage 2 Control Plane WebSocket and executing synchronized birthday/paced UDP connectivity checks before upgrading the winning socket pair to Quinn QUIC mTLS.

---

## 4. Acceptance Criteria (Measurable)

All criteria below must be quantitatively verified before marking `S3.P2.SP1` complete:

- [ ] `IceCandidateSet` gathering (Host + ServerReflexive) completes in `< 300 ms` when STUN server responds.
- [ ] Direct UDP hole punching succeeds in `100%` of simulated Full-Cone, Restricted-Cone, and Port-Restricted-Cone NAT topologies within `< 800 ms`.
- [ ] All connectivity probe packets carry HMAC-BLAKE3 short-term credentials exchanged over the authenticated control plane.
- [ ] Zero unresolved placeholders (`scripts/check-no-placeholders.sh` exits `0`).
- [ ] Rootless Podman disk usage after build & test remains `< 29.0 GiB` (`scripts/podman-quota-check.sh` exits `0`).

---

## 5. Required Tests (Classified)

Every test category below must be implemented, executed, and logged in the subphase evidence bundle:

- **Unit Tests:**
  - Test RFC 8489 STUN packet parser/encoder against official RFC test vectors (`XOR-MAPPED-ADDRESS` IPv4 & IPv6).
- **Integration Tests:**
  - Run embedded STUN server + two NAT simulator proxies (`PortRestrictedCone` <-> `PortRestrictedCone`) and verify direct QUIC mTLS establishment via hole punching.
- **Simulation / Fault-Injection Tests:**
  - Simulate `SymmetricNat` <-> `SymmetricNat` topology and verify clean detection (`NatTraversalResult::RequiresRelay`) within `1200 ms` timeout.
- **Hardware-Required Tests:**
  - Not Required.
- **Manually Verified Tests:**
  - Inspect gathered ICE candidate priority list and NAT type diagnostic output.

---

## 6. Required Evidence & Artifacts

Generate and archive the following deterministic artifacts before completing `S3.P2.SP1`:

- `artifacts/evidence/stage-03/S3_P2_SP1_evidence.json`
- `artifacts/evidence/stage-03/nat_hole_punch_matrix.json`
- Standard Evidence Schema (`artifacts/evidence/stage-03/S3_P2_SP1_evidence.json`):
  ```json
  {
    "subphase": "S3.P2.SP1",
    "title": "STUN/ICE Candidate Gathering & UDP Hole Punching",
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

1. **Module & Rustdoc Documentation:** Document all public structs, traits, functions, error codes, and wire schemas added in `S3.P2.SP1`.
2. **`TODO.md` Update:** Flip `- [ ] S3.P2.SP1` to `- [x] S3.P2.SP1` in `TODO.md` and record the evidence file path.
3. **`CHANGELOG.md` Update:** Append a bullet under `[Unreleased] -> ### Stage 3 — Phase 2` summarizing the concrete deliverables and metrics of `S3.P2.SP1`.

---

## 8. Rollback Criteria & Completion Gate

### Rollback Criteria
Immediately revert `S3.P2.SP1` changes (`git reset --hard` to the `S3.P1.SP3` checkpoint tag) and prune container caches if any of the following occur:
1. `scripts/podman-quota-check.sh` exceeds `29.0 GiB` and automated `cargo sweep` / `podman system prune` cannot recover below `25.0 GiB`.
2. Any prior stage or subphase test suite regresses.
3. Wire protocol backward compatibility in `umcf-proto` is broken without an explicit major version negotiation path.

### Completion Gate
`S3.P2.SP1` is **COMPLETE** if and only if:
- All implementation tasks are committed with zero placeholders;
- Unit, Integration, and Simulation tests pass 100% inside Podman;
- `artifacts/evidence/stage-03/S3_P2_SP1_evidence.json` is written and validated;
- `TODO.md` and `CHANGELOG.md` are updated.
