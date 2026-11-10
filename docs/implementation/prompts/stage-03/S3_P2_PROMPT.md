# Executable Phase Prompt — `S3.P2`: NAT Traversal, STUN/ICE Hole Punching & Relay Fallback

> **Stage:** Stage 3 — LAN/Remote Networking, Security, Hardware Discovery and Benchmarking (`STAGE_03_PROMPT.md`)  
> **Phase:** `S3.P2` — NAT Traversal, STUN/ICE Hole Punching & Relay Fallback  
> **Subphases:** `S3.P2.SP1`, `S3.P2.SP2`, `S3.P2.SP3`  
> **Target Environment:** Rootless Podman (`<= 30 GB` Hard Disk Ceiling)

---

## 1. Role & Context

You are the Lead Systems Architect executing Phase **`S3.P2` (NAT Traversal, STUN/ICE Hole Punching & Relay Fallback)** of Stage 3 in the Universal Distributed Mobile-First Compute Fabric (**UMCF**).

- **Phase Objective:** Enable seamless remote WAN connectivity across NATs and firewalls using RFC 8489 STUN, ICE-lite candidate gathering, synchronized UDP hole punching coordinated via the control plane, encrypted DERP/TURN relay fallback, and QUIC connection migration across network interfaces.
- **Execution Discipline:** Execute `S3.P2.SP1`, `S3.P2.SP2`, and `S3.P2.SP3` sequentially. Do not begin a subsequent subphase until the current subphase's Completion Gate and Evidence JSON are verified.

---

## 2. Prerequisites & Dependencies

- **Upstream Dependency:** `S3.P1` must be 100% complete and certified.
- **Target Crates / Services:** `crates/umcf-net, crates/umcf-hwprobe`
- **Quota Enforcement:** `scripts/podman-quota-check.sh` must pass before and after each subphase (`< 29.0 GiB`).

---

## 3. Implementation Tasks & Subphase Execution Sequence

Execute the three constituent subphases in strict sequential order:

### 3.1 Subphase `S3.P2.SP1`: STUN/ICE Candidate Gathering & UDP Hole Punching
- **Prompt File:** [`S3_P2_SP1.md`](S3_P2_SP1.md)
- **Scope:** Implement RFC 8489 STUN binding requests, NAT mapping/filtering behavior classification (Endpoint-Independent, Address-Dependent, Symmetric), ICE candidate gathering (`Host`, `ServerReflexive`, `PeerReflexive`), and synchronized UDP hole punching in `crates/umcf-net/src/nat.rs`.
- **Key Deliverables:**
  - Implement `crates/umcf-net/src/stun.rs` encoding/decoding RFC 8489 STUN `BindingRequest` and `BindingResponse` (`XOR-MAPPED-ADDRESS`, `FINGERPRINT`, `SOFTWARE`) and querying multiple STUN endpoints concurrently.
  - Implement `NatClassifier` in `crates/umcf-net/src/nat.rs` detecting `OpenInternet`, `FullCone / EndpointIndependent`, `RestrictedCone`, `PortRestrictedCone`, or `SymmetricNat`.
  - Implement `HolePunchCoordinator` exchanging signed `IceCandidateSet` payloads over the Stage 2 Control Plane WebSocket and executing synchronized birthday/paced UDP connectivity checks before upgrading the winning socket pair to Quinn QUIC mTLS.

### 3.2 Subphase `S3.P2.SP2`: Encrypted TURN/DERP-Style Relay Fallback Protocol
- **Prompt File:** [`S3_P2_SP2.md`](S3_P2_SP2.md)
- **Scope:** Implement zero-configuration fallback to an encrypted, zero-knowledge packet relay (`UmcfRelayServer` & `RelayPacketTransport`, inspired by Tailscale DERP / TURN over QUIC/WebSocket) so nodes behind strict Symmetric NATs or enterprise UDP-blocking firewalls always connect.
- **Key Deliverables:**
  - Implement `crates/umcf-net/src/relay.rs` defining `RelayFrame` (`BindNodeDid`, `SendPacket { dst_did, encrypted_quic_datagram }`, `RecvPacket { src_did, encrypted_quic_datagram }`, `KeepAlive`) over both UDP/QUIC and HTTPS/WebSocket (port 443 firewall bypass).
  - Implement custom `quinn::AsyncUdpSocket` adapter (`HybridUdpRelaySocket`) that transparently multiplexes direct UDP datagrams and relay-encapsulated datagrams so the upper Quinn QUIC mTLS session remain end-to-end encrypted (`Zero-Knowledge Relay` invariant: the relay sees only outer `src_did -> dst_did` and opaque QUIC ciphertext).
  - Implement background direct-path upgrade probing (`PathUpgradeMonitor`) that seamlessly transitions an active relayed connection to direct P2P UDP if a hole punch later succeeds.

### 3.3 Subphase `S3.P2.SP3`: Roaming, Network Interface Handover & Connection Migration
- **Prompt File:** [`S3_P2_SP3.md`](S3_P2_SP3.md)
- **Scope:** Implement proactive network interface monitoring (Wi-Fi <-> Cellular <-> Ethernet handover, IPv4 <-> IPv6 change) and RFC 9000 QUIC Connection Migration (`PATH_CHALLENGE` / `PATH_RESPONSE`) so active jobs and streams survive mobile roaming without disconnecting.
- **Key Deliverables:**
  - Implement `crates/umcf-net/src/roaming.rs` (`NetworkInterfaceWatcher`) polling/listening for local IP address changes, interface up/down events, and Wi-Fi -> Cellular metric changes.
  - Implement active socket rebinding (`Endpoint::rebind`) and RFC 9000 QUIC connection migration in `QuicTransportEngine` so existing `ConnectionId` sessions migrate to the new local IP/port in `< 250 ms`.
  - Publish updated `IceCandidateSet` to the cluster registry upon interface change so newly joining peers also know the node's updated endpoint.

---

## 4. Acceptance Criteria (Phase-Level Measurable Gates)

- [ ] All acceptance criteria in `S3_P2_SP1.md`, `S3_P2_SP2.md`, and `S3_P2_SP3.md` are satisfied.
- [ ] Cross-subphase integration between `S3.P2.SP1`, `S3.P2.SP2`, and `S3.P2.SP3` compiles cleanly with zero warnings (`cargo clippy --workspace --all-targets -- -D warnings`).
- [ ] Line coverage across crates modified in `S3.P2` is `>= 85.0%`.
- [ ] Zero `todo!()`, `unimplemented!()`, or placeholder stubs exist in the codebase (`scripts/check-no-placeholders.sh` exits `0`).
- [ ] Podman storage utilization remains strictly `< 29.0 GiB`.

---

## 5. Required Tests (Classified Rollup)

- **Unit Tests:** All unit test suites across `S3.P2.SP1`, `S3.P2.SP2`, and `S3.P2.SP3` pass via `cargo nextest run`.
- **Integration Tests:** End-to-end phase integration tests verifying `STUN/ICE Candidate Gathering & UDP Hole Punching` + `Encrypted TURN/DERP-Style Relay Fallback Protocol` + `Roaming, Network Interface Handover & Connection Migration` interoperation.
- **Simulation Tests:** Fault-injection, corrupted input, and concurrency stress tests defined in `S3.P2.SP1..SP3`.
- **Hardware-Required Tests:** Executed on target hardware or recorded in hardware execution manifests where containerized hardware passthrough is unavailable.
- **Manually Verified Tests:** Developer visual/CLI inspection checklists completed for `S3.P2.SP1..SP3`.

---

## 6. Required Evidence & Artifacts

- `artifacts/evidence/stage-03/S3_P2_SP1_evidence.json`
- `artifacts/evidence/stage-03/S3_P2_SP2_evidence.json`
- `artifacts/evidence/stage-03/S3_P2_SP3_evidence.json`
- `artifacts/evidence/stage-03/S3_P2_PHASE_SUMMARY.json` aggregating all 3 subphase evidence bundles and phase-level coverage metrics.

---

## 7. Documentation, `TODO.md` & `CHANGELOG.md` Updates

- Verify `S3.P2.SP1`, `S3.P2.SP2`, and `S3.P2.SP3` are all checked `[x]` in `TODO.md`.
- Consolidate `S3.P2` capabilities into `CHANGELOG.md` and module-level architecture docs.

---

## 8. Rollback Criteria & Completion Gate

- **Rollback Criteria:** If phase integration tests fail or Podman disk quota exceeds `29.0 GiB` after cache pruning, roll back to the start of `S3.P2` (`v0.3.1-certified`) and isolate the failing subphase.
- **Completion Gate:** Tag repository checkpoint `v0.3.2-certified` once `S3_P2_PHASE_SUMMARY.json` reports `"status": "PASS"`.
