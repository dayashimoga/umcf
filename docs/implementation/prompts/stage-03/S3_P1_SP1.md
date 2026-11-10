# Executable Subphase Prompt — `S3.P1.SP1`: Zero-Conf mDNS/DNS-SD & Signed UDP Broadcast Discovery

> **Stage:** Stage 3 — LAN/Remote Networking, Security, Hardware Discovery and Benchmarking (`STAGE_03_PROMPT.md`)  
> **Phase:** Phase `S3.P1` — LAN Discovery & Direct P2P QUIC Data Plane (`S3_P1_PROMPT.md`)  
> **Subphase:** `S3.P1.SP1` — Zero-Conf mDNS/DNS-SD & Signed UDP Broadcast Discovery  
> **Execution Mode:** Single-Subphase Atomic Implementation inside Rootless Podman (`<= 30 GB` Hard Disk Ceiling)

---

## 1. Role & Context

You are a Principal Distributed Systems & Cross-Platform Rust/Flutter/Edge Architect executing Subphase **`S3.P1.SP1` (Zero-Conf mDNS/DNS-SD & Signed UDP Broadcast Discovery)** of the Universal Distributed Mobile-First Compute Fabric (**UMCF**).

- **Subphase Scope:** Implement `crates/umcf-net/src/discovery.rs` supporting dual-stack IPv4/IPv6 mDNS service advertisement (`_umcf-mesh._udp.local.`) and fallback signed UDP subnet broadcast/multicasting (`239.255.77.67:44767` / `ff02::umcf`) with rate-limiting and replay protection.
- **Architectural Mandate:** Implement production-grade, memory-safe, deterministic code with zero `todo!()`, `unimplemented!()`, or mock-only placeholders in production crates/services. All build, lint, and test executions must run inside rootless Podman containers while enforcing the `<= 30 GB` disk ceiling via `scripts/podman-quota-check.sh`.

---

## 2. Prerequisites & Dependencies

- **Upstream Gate:** `S2.P3.SP3` must be marked `[x]` in `TODO.md` with verified evidence JSON on disk.
- Stage 2 complete (`S2.P3.SP3`)
- **Podman Quota Pre-Check:** Run `scripts/podman-quota-check.sh` before starting implementation; total footprint must be `< 29.0 GiB` (`31,138,512,896` bytes).

---

## 3. Implementation Tasks

Execute the following concrete implementation tasks in order:

1. Create `crates/umcf-net` and implement `LanDiscoveryEngine` in `src/discovery.rs` advertising `_umcf-mesh._udp.local.` TXT records (`v=1`, `cid=<cluster_id_prefix>`, `did=<did:key>`, `role=<Master|Worker|Hybrid>`, `port=<quic_port>`).
2. Implement `SignedDiscoveryBeacon` transmitted over UDP multicast/broadcast containing `ClusterId`, `SenderDid`, `HlcTimestamp`, `listen_addrs: SmallVec<[SocketAddr; 4]>`, `capability_digest: [u8; 32]`, and Ed25519 signature.
3. Implement token-bucket per-IP rate limiting (`max 10 beacons/sec/peer`) and replay window filtering so spoofed or replayed LAN beacons are dropped before waking the QUIC connector.

---

## 4. Acceptance Criteria (Measurable)

All criteria below must be quantitatively verified before marking `S3.P1.SP1` complete:

- [ ] Two UMCF nodes on the same LAN/virtual bridge discover each other and validate `SignedDiscoveryBeacon` within `< 500 ms`.
- [ ] Unsigned, malformed, or foreign-cluster beacons (`ClusterId` mismatch) are dropped in `< 50 µs`.
- [ ] Beacon flood of `5,000 packets/sec` from a rogue IP is throttled by the token-bucket limiter without exceeding `2%` CPU utilization.
- [ ] Zero unresolved placeholders (`scripts/check-no-placeholders.sh` exits `0`).
- [ ] Rootless Podman disk usage after build & test remains `< 29.0 GiB` (`scripts/podman-quota-check.sh` exits `0`).

---

## 5. Required Tests (Classified)

Every test category below must be implemented, executed, and logged in the subphase evidence bundle:

- **Unit Tests:**
  - Test `SignedDiscoveryBeacon` serialization, TXT record parsing, Ed25519 verification, and token-bucket rate limiter.
- **Integration Tests:**
  - Spawn 3 `LanDiscoveryEngine` instances bound to loopback UDP multicast ports and verify full mesh peer table convergence within `500 ms`.
- **Simulation / Fault-Injection Tests:**
  - Flood a discovery listener with 10,000 forged/corrupted UDP packets and verify zero panics and bounded memory (`< 4 MiB`).
- **Hardware-Required Tests:**
  - Optional multi-device Wi-Fi LAN discovery test; loopback multicast executes in Podman.
- **Manually Verified Tests:**
  - Inspect discovered peer table output and TXT record formatting.

---

## 6. Required Evidence & Artifacts

Generate and archive the following deterministic artifacts before completing `S3.P1.SP1`:

- `artifacts/evidence/stage-03/S3_P1_SP1_evidence.json`
- `artifacts/evidence/stage-03/lan_discovery_convergence.json`
- Standard Evidence Schema (`artifacts/evidence/stage-03/S3_P1_SP1_evidence.json`):
  ```json
  {
    "subphase": "S3.P1.SP1",
    "title": "Zero-Conf mDNS/DNS-SD & Signed UDP Broadcast Discovery",
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

1. **Module & Rustdoc Documentation:** Document all public structs, traits, functions, error codes, and wire schemas added in `S3.P1.SP1`.
2. **`TODO.md` Update:** Flip `- [ ] S3.P1.SP1` to `- [x] S3.P1.SP1` in `TODO.md` and record the evidence file path.
3. **`CHANGELOG.md` Update:** Append a bullet under `[Unreleased] -> ### Stage 3 — Phase 1` summarizing the concrete deliverables and metrics of `S3.P1.SP1`.

---

## 8. Rollback Criteria & Completion Gate

### Rollback Criteria
Immediately revert `S3.P1.SP1` changes (`git reset --hard` to the `S2.P3.SP3` checkpoint tag) and prune container caches if any of the following occur:
1. `scripts/podman-quota-check.sh` exceeds `29.0 GiB` and automated `cargo sweep` / `podman system prune` cannot recover below `25.0 GiB`.
2. Any prior stage or subphase test suite regresses.
3. Wire protocol backward compatibility in `umcf-proto` is broken without an explicit major version negotiation path.

### Completion Gate
`S3.P1.SP1` is **COMPLETE** if and only if:
- All implementation tasks are committed with zero placeholders;
- Unit, Integration, and Simulation tests pass 100% inside Podman;
- `artifacts/evidence/stage-03/S3_P1_SP1_evidence.json` is written and validated;
- `TODO.md` and `CHANGELOG.md` are updated.
