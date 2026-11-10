---
name: umcf-networking-p2p
description: >-
  Guides P2P QUIC mesh networking, LAN mDNS discovery, STUN/TURN NAT traversal, connection migration, stream multiplexing, and ultra-low-latency remote desktop/game streaming protocols in UMCF. Use when working on umcf-p2p, umcf-stream-engine, or NAT signaling.
---

# UMCF P2P QUIC Networking & Low-Latency Streaming Skill

Use this skill when developing or debugging peer discovery, NAT traversal, QUIC transport multiplexing, or remote desktop/game streaming in `umcf-p2p` and `umcf-stream-engine`.

---

## 1. Multi-Tier Peer Discovery & NAT Traversal

UMCF nodes discover and connect to peers through a deterministic three-tier waterfall:

1. **Tier 1: Zero-Conf Local Area Network (LAN / Wi-Fi Direct)**:
   - Broadcast and listen for mDNS / DNS-SD service records (`_umcf._udp.local`) containing the node's `NodeId` fingerprint, QUIC UDP port, and protocol version.
   - Direct LAN connections bypass internet NATs entirely, achieving < 2 ms RTT.
2. **Tier 2: Direct WAN P2P via STUN & Simultaneous UDP Hole Punching**:
   - Query control-plane or public/self-hosted STUN servers (`RFC 5389`) to discover reflexive `SocketAddr` candidates and classify NAT behavior (Endpoint-Independent vs Symmetric).
   - Exchange candidate lists through the authenticated Cloudflare Durable Object signaling channel and execute synchronized UDP hole punching before establishing the QUIC handshake.
3. **Tier 3: Encrypted TURN / QUIC Relay Fallback**:
   - When both peers are behind strict Symmetric NATs or carrier-grade NAT (CGNAT) where UDP hole punching fails after bounded retries (e.g., 2.5 seconds), fall back to an explicitly authorized, end-to-end encrypted relay path (peers still perform end-to-end mTLS/Noise authentication so the relay only sees ciphertext).

---

## 2. QUIC Transport Configuration (`quinn` + `rustls`)

1. **Multiplexed Stream & Datagram Channels**:
   Over a single authenticated QUIC connection between two peers, segregate traffic by stream priority and delivery semantics:
   - **Control & RPC Bi-Streams (High Priority)**: Capability negotiation, stage leases, heartbeats, remote input events (keyboard/mouse/gamepad).
   - **Inference Pipeline Uni/Bi-Streams (High Priority)**: Sequential tensor activation frames (`TCP-like` ordered delivery per stream, zero head-of-line blocking across independent inference sessions).
   - **Model Shard Bulk Transfer Streams (Low Priority)**: Content-addressed GGUF layer shard downloads with bandwidth pacing so bulk transfers never starve active inference or interactive streaming.
   - **QUIC Unreliable Datagrams (`RFC 9221`)**: Real-time remote desktop/game audio and non-IDR delta video frames where retransmitting stale frames is worse than dropping or requesting an instant IDR/keyframe.
2. **Connection Migration**:
   - Enable QUIC connection migration so mobile nodes switching between Wi-Fi and 5G/LTE retain their active session state without tearing down upper-layer pipeline streams.

---

## 3. Remote Desktop & Game Streaming Protocol (`umcf-stream-engine`)

1. **Capture & Hardware Encoding Pipeline**:
   - Capture display frames using native zero-copy OS APIs where available (Windows DXGI Desktop Duplication, Linux PipeWire/DRM, macOS ScreenCaptureKit, Android MediaProjection).
   - Encode via hardware encoders (`H.264` / `HEVC` / `AV1`) tuned for ultra-low latency (`zerolatency` profile, zero B-frames, intra-refresh or on-demand IDR upon packet loss feedback).
2. **Adaptive Bitrate & Congestion Control**:
   - Monitor QUIC RTT gradient, packet loss rate, and receiver decode queue depth.
   - Dynamically adjust target encoder bitrate (e.g., 2 Mbps to 50 Mbps) and resolution/framerate (30/60/120 FPS) within 100 ms of congestion onset.
3. **Input Injection & Local User Override**:
   - Transmit mouse movement, clicks, scroll, keyboard scan codes, and gamepad axes over a high-priority QUIC stream with monotonic sequence numbers and timestamp anti-replay checks.
   - Enforce an immediate hardware override on the host: physical local keyboard/mouse activity can pause or terminate remote input injection at any time.
