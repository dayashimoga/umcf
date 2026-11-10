# ADR-0004: P2P QUIC Transport (`quinn`), Simultaneous NAT Hole-Punching, and Encrypted Relay Fallback

- **Status**: Accepted
- **Date**: 2026-11-10
- **Deciders**: UMCF Core Architecture Team
- **Related Components**: `umcf-core`, `umcf-proto`, `umcf-engine-stream`, `services/control-plane-cf`

---

## 1. Context and Problem Statement

UMCF's Data Plane simultaneously carries multiple traffic classes between peer nodes:
1. Reliable ordered control RPCs (`TaskLeaseAssign`, `HeartbeatPing`)
2. Latency-critical reliable tensor streams (`TensorRpcForward`)
3. Unreliable real-time media datagrams (`120fps` video slices, high-frequency mouse motion)
4. Bulk background transfers (`4 MiB` BLAKE3 CAS model weight chunks)

Over standard TCP, a single dropped packet in a background 4 MiB CAS chunk transfer triggers kernel-level **Head-of-Line (HoL) blocking**, stalling concurrent real-time inference tensors or game streaming packets sharing the connection. Furthermore, opening multiple separate TCP/UDP ports complicates NAT traversal across home routers and mobile carrier networks.

## 2. Decision Drivers

1. **Zero Head-of-Line Blocking Across Streams**: Independent streams and unreliable datagrams must multiplex over a single UDP port (`:4433` default).
2. **Mandatory End-to-End Encryption with Node Identity Pinning**: Every packet must be authenticated via the peer's Ed25519 `NodeId` keypair.
3. **High NAT Traversal Success Rate**: Must connect directly across home Wi-Fi (`mDNS`) and most WAN NATs (`STUN` + UDP hole-punching), with guaranteed fallback (`TURN` / DERP-style QUIC relay) when UDP is blocked or both peers are behind Symmetric CGNAT.
4. **Connection Migration**: Seamless roaming when a mobile device switches from Wi-Fi to 5G without tearing down active compute leases.

## 3. Decision Outcome

We standardize the UMCF Data Plane on **QUIC (RFC 9000) + Unreliable QUIC Datagrams (RFC 9221)** implemented via the pure-Rust **`quinn`** and **`rustls`** crates (with **WebTransport / WebRTC DataChannels** bridging browser nodes).

### 3.1 Connection Establishment Hierarchy
1. **LAN Fast-Path (`< 5ms`)**: Continuous multicast DNS (`_umcf._udp.local`) discovery exchanges local IPv4/IPv6 socket addresses. Peers establish direct LAN QUIC connections immediately without external STUN/Control Plane dependency.
2. **WAN Direct Hole-Punching (`<= 600ms` Budget)**: Peers discover their server-reflexive address (`XOR-MAPPED-ADDRESS`) via STUN, exchange candidates through the Control Plane WebSocket, and transmit synchronized QUIC Initial / UDP probe packets across candidate pairs.
3. **Happy-Eyeballs Encrypted Relay Fallback (`<= 900ms` Total)**: At `t = 250ms` into WAN hole-punching, `umcf-core` prepares a standby relay allocation via a designated Server Node or TURN/DERP relay. If direct UDP hole-punching does not complete by `t = 600ms`, traffic flows immediately over the relay while background hole-punch probes continue every `30s` for transparent QUIC connection migration.

### 3.2 Stream Priority & Congestion Control
- **Stream Priorities**: QUIC stream priority levels (`0 = Control`, `1 = Input`, `2 = AI Tensor / Realtime Media`, `3 = Transcode`, `4 = CAS Bulk`) ensure bulk model downloads never starve interactive inference or streaming packets.
- **End-to-End mTLS over Relays**: When traversing a relay, peers encapsulate their mTLS QUIC session inside the relay circuit (QUIC-in-QUIC or encrypted datagram forwarding); the relay node sees only opaque ciphertext and cannot inspect payloads.

## 4. Consequences

### Positive
- **Single UDP Socket per Node**: Simplifies firewall rules, STUN hole-punching, and socket lifecycle management across Android, iOS, and desktop OSes.
- **Unified Reliable + Unreliable Transport**: Supports both lossless `llama.cpp` tensor streams and loss-tolerant 120fps video datagrams on the same authenticated session.

### Negative & Mitigations
- **UDP Throttling by Some Enterprise/Hotel Networks**: Some captive networks block all UDP traffic. *Mitigation*: The relay fallback supports tunneling encrypted frames over **WebSocket over TLS (`WSS` on TCP port 443)** when UDP is completely blocked.
