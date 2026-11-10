# Executable Subphase Prompt — `S3.P1.SP2`: Quinn/QUIC Multiplexed Data-Plane Transport with mTLS

> **Stage:** Stage 3 — LAN/Remote Networking, Security, Hardware Discovery and Benchmarking (`STAGE_03_PROMPT.md`)  
> **Phase:** Phase `S3.P1` — LAN Discovery & Direct P2P QUIC Data Plane (`S3_P1_PROMPT.md`)  
> **Subphase:** `S3.P1.SP2` — Quinn/QUIC Multiplexed Data-Plane Transport with mTLS  
> **Execution Mode:** Single-Subphase Atomic Implementation inside Rootless Podman (`<= 30 GB` Hard Disk Ceiling)

---

## 1. Role & Context

You are a Principal Distributed Systems & Cross-Platform Rust/Flutter/Edge Architect executing Subphase **`S3.P1.SP2` (Quinn/QUIC Multiplexed Data-Plane Transport with mTLS)** of the Universal Distributed Mobile-First Compute Fabric (**UMCF**).

- **Subphase Scope:** Implement the high-throughput, low-latency `QuicTransportEngine` in `crates/umcf-net/src/quic.rs` built on `quinn` and `rustls` TLS 1.3 with mandatory client/server mutual authentication (`NodeCertificate`), ALPN `umcf-quic/v1`, 0-RTT session resumption within cluster epoch, and connection pooling.
- **Architectural Mandate:** Implement production-grade, memory-safe, deterministic code with zero `todo!()`, `unimplemented!()`, or mock-only placeholders in production crates/services. All build, lint, and test executions must run inside rootless Podman containers while enforcing the `<= 30 GB` disk ceiling via `scripts/podman-quota-check.sh`.

---

## 2. Prerequisites & Dependencies

- **Upstream Gate:** `S3.P1.SP1` must be marked `[x]` in `TODO.md` with verified evidence JSON on disk.
- `S3.P1.SP1` completed
- **Podman Quota Pre-Check:** Run `scripts/podman-quota-check.sh` before starting implementation; total footprint must be `< 29.0 GiB` (`31,138,512,896` bytes).

---

## 3. Implementation Tasks

Execute the following concrete implementation tasks in order:

1. Implement `crates/umcf-net/src/quic.rs` configuring `quinn::Endpoint` with `rustls` TLS 1.3 mTLS enforcing `ClusterCaVerifier`, custom SAN `did:key` extraction, and `ClusterCrl` revocation check during handshake.
2. Configure BBRv2 / Cubic congestion controller, adaptive UDP socket receive/send buffers (`4 MiB`–`16 MiB`), keep-alive intervals (`5s`), and stream multiplexing (`BidirectionalControl`, `BidirectionalRpc`, `UnidirectionalBulkTensor`, `UnreliableDatagramMedia`).
3. Bind `MutualAttestationProtocol` (`S2.P1.SP3`) immediately upon connection establishment before promoting a `PeerConnection` to `AuthenticatedReady`.

---

## 4. Acceptance Criteria (Measurable)

All criteria below must be quantitatively verified before marking `S3.P1.SP2` complete:

- [ ] QUIC mTLS 1-RTT handshake + mutual attestation completes in `< 15 ms` on LAN loopback.
- [ ] Connections from peers with revoked certificates in `ClusterCrl` or belonging to a different `ClusterId` are terminated at TLS handshake with `ALERT_CERTIFICATE_REVOKED` / `ALERT_ACCESS_DENIED`.
- [ ] Sustained bulk unidirectional stream throughput over loopback reaches `>= 8.0 Gbps` (or `>= 85%` of virtual link capacity) with zero head-of-line blocking across independent streams.
- [ ] Zero unresolved placeholders (`scripts/check-no-placeholders.sh` exits `0`).
- [ ] Rootless Podman disk usage after build & test remains `< 29.0 GiB` (`scripts/podman-quota-check.sh` exits `0`).

---

## 5. Required Tests (Classified)

Every test category below must be implemented, executed, and logged in the subphase evidence bundle:

- **Unit Tests:**
  - Test `ClusterCaVerifier` SAN extraction, ALPN `umcf-quic/v1` enforcement, and stream type header routing.
- **Integration Tests:**
  - Establish multiplexed QUIC connection between two nodes, open 64 concurrent bidirectional RPC streams + 4 bulk tensor streams, and verify integrity via BLAKE3.
- **Simulation / Fault-Injection Tests:**
  - Attempt connection with expired cert, revoked cert in `ClusterCrl`, and self-signed untrusted cert; verify all 3 fail handshake.
- **Hardware-Required Tests:**
  - Not Required.
- **Manually Verified Tests:**
  - Review QUIC transport parameters and congestion window telemetry.

---

## 6. Required Evidence & Artifacts

Generate and archive the following deterministic artifacts before completing `S3.P1.SP2`:

- `artifacts/evidence/stage-03/S3_P1_SP2_evidence.json`
- `artifacts/evidence/stage-03/quic_throughput_benchmark.json`
- Standard Evidence Schema (`artifacts/evidence/stage-03/S3_P1_SP2_evidence.json`):
  ```json
  {
    "subphase": "S3.P1.SP2",
    "title": "Quinn/QUIC Multiplexed Data-Plane Transport with mTLS",
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

1. **Module & Rustdoc Documentation:** Document all public structs, traits, functions, error codes, and wire schemas added in `S3.P1.SP2`.
2. **`TODO.md` Update:** Flip `- [ ] S3.P1.SP2` to `- [x] S3.P1.SP2` in `TODO.md` and record the evidence file path.
3. **`CHANGELOG.md` Update:** Append a bullet under `[Unreleased] -> ### Stage 3 — Phase 1` summarizing the concrete deliverables and metrics of `S3.P1.SP2`.

---

## 8. Rollback Criteria & Completion Gate

### Rollback Criteria
Immediately revert `S3.P1.SP2` changes (`git reset --hard` to the `S3.P1.SP1` checkpoint tag) and prune container caches if any of the following occur:
1. `scripts/podman-quota-check.sh` exceeds `29.0 GiB` and automated `cargo sweep` / `podman system prune` cannot recover below `25.0 GiB`.
2. Any prior stage or subphase test suite regresses.
3. Wire protocol backward compatibility in `umcf-proto` is broken without an explicit major version negotiation path.

### Completion Gate
`S3.P1.SP2` is **COMPLETE** if and only if:
- All implementation tasks are committed with zero placeholders;
- Unit, Integration, and Simulation tests pass 100% inside Podman;
- `artifacts/evidence/stage-03/S3_P1_SP2_evidence.json` is written and validated;
- `TODO.md` and `CHANGELOG.md` are updated.
