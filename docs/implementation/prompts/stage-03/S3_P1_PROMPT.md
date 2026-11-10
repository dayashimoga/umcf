# Executable Phase Prompt — `S3.P1`: LAN Discovery & Direct P2P QUIC Data Plane

> **Stage:** Stage 3 — LAN/Remote Networking, Security, Hardware Discovery and Benchmarking (`STAGE_03_PROMPT.md`)  
> **Phase:** `S3.P1` — LAN Discovery & Direct P2P QUIC Data Plane  
> **Subphases:** `S3.P1.SP1`, `S3.P1.SP2`, `S3.P1.SP3`  
> **Target Environment:** Rootless Podman (`<= 30 GB` Hard Disk Ceiling)

---

## 1. Role & Context

You are the Lead Systems Architect executing Phase **`S3.P1` (LAN Discovery & Direct P2P QUIC Data Plane)** of Stage 3 in the Universal Distributed Mobile-First Compute Fabric (**UMCF**).

- **Phase Objective:** Implement sub-second LAN peer discovery via mDNS/DNS-SD and signed UDP broadcast beacons, multiplexed Quinn QUIC transport with mutual TLS (`NodeCertificate`), and strict traffic isolation/QoS between control-plane and data-plane streams.
- **Execution Discipline:** Execute `S3.P1.SP1`, `S3.P1.SP2`, and `S3.P1.SP3` sequentially. Do not begin a subsequent subphase until the current subphase's Completion Gate and Evidence JSON are verified.

---

## 2. Prerequisites & Dependencies

- **Upstream Dependency:** `Stage 2` must be 100% complete and certified.
- **Target Crates / Services:** `crates/umcf-net, crates/umcf-hwprobe`
- **Quota Enforcement:** `scripts/podman-quota-check.sh` must pass before and after each subphase (`< 29.0 GiB`).

---

## 3. Implementation Tasks & Subphase Execution Sequence

Execute the three constituent subphases in strict sequential order:

### 3.1 Subphase `S3.P1.SP1`: Zero-Conf mDNS/DNS-SD & Signed UDP Broadcast Discovery
- **Prompt File:** [`S3_P1_SP1.md`](S3_P1_SP1.md)
- **Scope:** Implement `crates/umcf-net/src/discovery.rs` supporting dual-stack IPv4/IPv6 mDNS service advertisement (`_umcf-mesh._udp.local.`) and fallback signed UDP subnet broadcast/multicasting (`239.255.77.67:44767` / `ff02::umcf`) with rate-limiting and replay protection.
- **Key Deliverables:**
  - Create `crates/umcf-net` and implement `LanDiscoveryEngine` in `src/discovery.rs` advertising `_umcf-mesh._udp.local.` TXT records (`v=1`, `cid=<cluster_id_prefix>`, `did=<did:key>`, `role=<Master|Worker|Hybrid>`, `port=<quic_port>`).
  - Implement `SignedDiscoveryBeacon` transmitted over UDP multicast/broadcast containing `ClusterId`, `SenderDid`, `HlcTimestamp`, `listen_addrs: SmallVec<[SocketAddr; 4]>`, `capability_digest: [u8; 32]`, and Ed25519 signature.
  - Implement token-bucket per-IP rate limiting (`max 10 beacons/sec/peer`) and replay window filtering so spoofed or replayed LAN beacons are dropped before waking the QUIC connector.

### 3.2 Subphase `S3.P1.SP2`: Quinn/QUIC Multiplexed Data-Plane Transport with mTLS
- **Prompt File:** [`S3_P1_SP2.md`](S3_P1_SP2.md)
- **Scope:** Implement the high-throughput, low-latency `QuicTransportEngine` in `crates/umcf-net/src/quic.rs` built on `quinn` and `rustls` TLS 1.3 with mandatory client/server mutual authentication (`NodeCertificate`), ALPN `umcf-quic/v1`, 0-RTT session resumption within cluster epoch, and connection pooling.
- **Key Deliverables:**
  - Implement `crates/umcf-net/src/quic.rs` configuring `quinn::Endpoint` with `rustls` TLS 1.3 mTLS enforcing `ClusterCaVerifier`, custom SAN `did:key` extraction, and `ClusterCrl` revocation check during handshake.
  - Configure BBRv2 / Cubic congestion controller, adaptive UDP socket receive/send buffers (`4 MiB`–`16 MiB`), keep-alive intervals (`5s`), and stream multiplexing (`BidirectionalControl`, `BidirectionalRpc`, `UnidirectionalBulkTensor`, `UnreliableDatagramMedia`).
  - Bind `MutualAttestationProtocol` (`S2.P1.SP3`) immediately upon connection establishment before promoting a `PeerConnection` to `AuthenticatedReady`.

### 3.3 Subphase `S3.P1.SP3`: Control-Plane vs Data-Plane Traffic Isolation & QoS
- **Prompt File:** [`S3_P1_SP3.md`](S3_P1_SP3.md)
- **Scope:** Implement strict priority scheduling, IP DSCP marking (`EF` / `CS6` for control & heartbeats, `AF41` for streaming, `AF21` / `BE` for bulk model/tensor transfers), and hierarchical token-bucket bandwidth shapers so bulk data transfers never starve cluster heartbeats or interactive streams.
- **Key Deliverables:**
  - Implement `crates/umcf-net/src/qos.rs` defining `TrafficClass`: `CriticalControl` (heartbeats, election, preemption signals; highest priority, reserved min `5%` bandwidth), `RealtimeInteractive` (remote streaming, input return, live LLM token stream), `ElasticCompute` (DAG task activations/RPC), and `BackgroundBulk` (GGUF shard sync, storage replication).
  - Configure `quinn::SendStream::set_priority` and user-space weighted fair / deficit round-robin pacing (`BandwidthShaper`) enforcing user-configured upload/download caps from `ResourceConsentPolicy`.
  - Implement OS socket DSCP / TOS byte marking (`IP_TOS` / `IPV6_TCLASS`) with graceful fallback on unprivileged sockets.

---

## 4. Acceptance Criteria (Phase-Level Measurable Gates)

- [ ] All acceptance criteria in `S3_P1_SP1.md`, `S3_P1_SP2.md`, and `S3_P1_SP3.md` are satisfied.
- [ ] Cross-subphase integration between `S3.P1.SP1`, `S3.P1.SP2`, and `S3.P1.SP3` compiles cleanly with zero warnings (`cargo clippy --workspace --all-targets -- -D warnings`).
- [ ] Line coverage across crates modified in `S3.P1` is `>= 85.0%`.
- [ ] Zero `todo!()`, `unimplemented!()`, or placeholder stubs exist in the codebase (`scripts/check-no-placeholders.sh` exits `0`).
- [ ] Podman storage utilization remains strictly `< 29.0 GiB`.

---

## 5. Required Tests (Classified Rollup)

- **Unit Tests:** All unit test suites across `S3.P1.SP1`, `S3.P1.SP2`, and `S3.P1.SP3` pass via `cargo nextest run`.
- **Integration Tests:** End-to-end phase integration tests verifying `Zero-Conf mDNS/DNS-SD & Signed UDP Broadcast Discovery` + `Quinn/QUIC Multiplexed Data-Plane Transport with mTLS` + `Control-Plane vs Data-Plane Traffic Isolation & QoS` interoperation.
- **Simulation Tests:** Fault-injection, corrupted input, and concurrency stress tests defined in `S3.P1.SP1..SP3`.
- **Hardware-Required Tests:** Executed on target hardware or recorded in hardware execution manifests where containerized hardware passthrough is unavailable.
- **Manually Verified Tests:** Developer visual/CLI inspection checklists completed for `S3.P1.SP1..SP3`.

---

## 6. Required Evidence & Artifacts

- `artifacts/evidence/stage-03/S3_P1_SP1_evidence.json`
- `artifacts/evidence/stage-03/S3_P1_SP2_evidence.json`
- `artifacts/evidence/stage-03/S3_P1_SP3_evidence.json`
- `artifacts/evidence/stage-03/S3_P1_PHASE_SUMMARY.json` aggregating all 3 subphase evidence bundles and phase-level coverage metrics.

---

## 7. Documentation, `TODO.md` & `CHANGELOG.md` Updates

- Verify `S3.P1.SP1`, `S3.P1.SP2`, and `S3.P1.SP3` are all checked `[x]` in `TODO.md`.
- Consolidate `S3.P1` capabilities into `CHANGELOG.md` and module-level architecture docs.

---

## 8. Rollback Criteria & Completion Gate

- **Rollback Criteria:** If phase integration tests fail or Podman disk quota exceeds `29.0 GiB` after cache pruning, roll back to the start of `S3.P1` (`v0.3.0-certified`) and isolate the failing subphase.
- **Completion Gate:** Tag repository checkpoint `v0.3.1-certified` once `S3_P1_PHASE_SUMMARY.json` reports `"status": "PASS"`.
