# Executable Subphase Prompt — `S3.P2.SP2`: Encrypted TURN/DERP-Style Relay Fallback Protocol

> **Stage:** Stage 3 — LAN/Remote Networking, Security, Hardware Discovery and Benchmarking (`STAGE_03_PROMPT.md`)  
> **Phase:** Phase `S3.P2` — NAT Traversal, STUN/ICE Hole Punching & Relay Fallback (`S3_P2_PROMPT.md`)  
> **Subphase:** `S3.P2.SP2` — Encrypted TURN/DERP-Style Relay Fallback Protocol  
> **Execution Mode:** Single-Subphase Atomic Implementation inside Rootless Podman (`<= 30 GB` Hard Disk Ceiling)

---

## 1. Role & Context

You are a Principal Distributed Systems & Cross-Platform Rust/Flutter/Edge Architect executing Subphase **`S3.P2.SP2` (Encrypted TURN/DERP-Style Relay Fallback Protocol)** of the Universal Distributed Mobile-First Compute Fabric (**UMCF**).

- **Subphase Scope:** Implement zero-configuration fallback to an encrypted, zero-knowledge packet relay (`UmcfRelayServer` & `RelayPacketTransport`, inspired by Tailscale DERP / TURN over QUIC/WebSocket) so nodes behind strict Symmetric NATs or enterprise UDP-blocking firewalls always connect.
- **Architectural Mandate:** Implement production-grade, memory-safe, deterministic code with zero `todo!()`, `unimplemented!()`, or mock-only placeholders in production crates/services. All build, lint, and test executions must run inside rootless Podman containers while enforcing the `<= 30 GB` disk ceiling via `scripts/podman-quota-check.sh`.

---

## 2. Prerequisites & Dependencies

- **Upstream Gate:** `S3.P2.SP1` must be marked `[x]` in `TODO.md` with verified evidence JSON on disk.
- `S3.P2.SP1` completed
- **Podman Quota Pre-Check:** Run `scripts/podman-quota-check.sh` before starting implementation; total footprint must be `< 29.0 GiB` (`31,138,512,896` bytes).

---

## 3. Implementation Tasks

Execute the following concrete implementation tasks in order:

1. Implement `crates/umcf-net/src/relay.rs` defining `RelayFrame` (`BindNodeDid`, `SendPacket { dst_did, encrypted_quic_datagram }`, `RecvPacket { src_did, encrypted_quic_datagram }`, `KeepAlive`) over both UDP/QUIC and HTTPS/WebSocket (port 443 firewall bypass).
2. Implement custom `quinn::AsyncUdpSocket` adapter (`HybridUdpRelaySocket`) that transparently multiplexes direct UDP datagrams and relay-encapsulated datagrams so the upper Quinn QUIC mTLS session remain end-to-end encrypted (`Zero-Knowledge Relay` invariant: the relay sees only outer `src_did -> dst_did` and opaque QUIC ciphertext).
3. Implement background direct-path upgrade probing (`PathUpgradeMonitor`) that seamlessly transitions an active relayed connection to direct P2P UDP if a hole punch later succeeds.

---

## 4. Acceptance Criteria (Measurable)

All criteria below must be quantitatively verified before marking `S3.P2.SP2` complete:

- [ ] When direct UDP hole punching is blocked (`SymmetricNat` or UDP firewall block), `HybridUdpRelaySocket` establishes a working end-to-end mTLS QUIC connection via `UmcfRelayServer` within `< 1500 ms`.
- [ ] The relay server possesses zero access to session keys or plaintext payloads (verified by end-to-end mTLS exporter binding across the relay).
- [ ] When direct UDP path becomes available mid-session, `PathUpgradeMonitor` switches traffic from `Relayed` to `DirectP2p` with `0` dropped QUIC streams.
- [ ] Zero unresolved placeholders (`scripts/check-no-placeholders.sh` exits `0`).
- [ ] Rootless Podman disk usage after build & test remains `< 29.0 GiB` (`scripts/podman-quota-check.sh` exits `0`).

---

## 5. Required Tests (Classified)

Every test category below must be implemented, executed, and logged in the subphase evidence bundle:

- **Unit Tests:**
  - Test `RelayFrame` codec, DID routing table in `UmcfRelayServer`, and per-client relay bandwidth quotas.
- **Integration Tests:**
  - Block all direct UDP traffic between Node A and Node B in virtual router, route through `UmcfRelayServer`, verify full QUIC mTLS RPC, then unblock direct UDP and verify automatic upgrade to `DirectP2p`.
- **Simulation / Fault-Injection Tests:**
  - Attempt to inspect or tamper with inner QUIC payload inside a malicious relay proxy and verify end-to-end QUIC AEAD immediately drops tampered datagrams.
- **Hardware-Required Tests:**
  - Not Required.
- **Manually Verified Tests:**
  - Inspect connection path status indicator (`DIRECT_LAN`, `DIRECT_WAN_PUNCHED`, `RELAYED_E2EE`).

---

## 6. Required Evidence & Artifacts

Generate and archive the following deterministic artifacts before completing `S3.P2.SP2`:

- `artifacts/evidence/stage-03/S3_P2_SP2_evidence.json`
- `artifacts/evidence/stage-03/relay_fallback_and_upgrade_trace.json`
- Standard Evidence Schema (`artifacts/evidence/stage-03/S3_P2_SP2_evidence.json`):
  ```json
  {
    "subphase": "S3.P2.SP2",
    "title": "Encrypted TURN/DERP-Style Relay Fallback Protocol",
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

1. **Module & Rustdoc Documentation:** Document all public structs, traits, functions, error codes, and wire schemas added in `S3.P2.SP2`.
2. **`TODO.md` Update:** Flip `- [ ] S3.P2.SP2` to `- [x] S3.P2.SP2` in `TODO.md` and record the evidence file path.
3. **`CHANGELOG.md` Update:** Append a bullet under `[Unreleased] -> ### Stage 3 — Phase 2` summarizing the concrete deliverables and metrics of `S3.P2.SP2`.

---

## 8. Rollback Criteria & Completion Gate

### Rollback Criteria
Immediately revert `S3.P2.SP2` changes (`git reset --hard` to the `S3.P2.SP1` checkpoint tag) and prune container caches if any of the following occur:
1. `scripts/podman-quota-check.sh` exceeds `29.0 GiB` and automated `cargo sweep` / `podman system prune` cannot recover below `25.0 GiB`.
2. Any prior stage or subphase test suite regresses.
3. Wire protocol backward compatibility in `umcf-proto` is broken without an explicit major version negotiation path.

### Completion Gate
`S3.P2.SP2` is **COMPLETE** if and only if:
- All implementation tasks are committed with zero placeholders;
- Unit, Integration, and Simulation tests pass 100% inside Podman;
- `artifacts/evidence/stage-03/S3_P2_SP2_evidence.json` is written and validated;
- `TODO.md` and `CHANGELOG.md` are updated.
