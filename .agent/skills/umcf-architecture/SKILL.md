---
name: umcf-architecture
description: >-
  Provides architectural patterns, system boundaries, wire protocol invariants, and control-plane vs data-plane separation rules for the Universal Mobile Compute Fabric (UMCF). Use when designing or modifying UMCF crates, services, cross-platform bridges, or system interfaces.
---

# UMCF System Architecture & Domain Boundaries Skill

Use this skill whenever designing, reviewing, or implementing architectural components, crate boundaries, wire protocols, or cross-platform integrations in the **Universal Mobile Compute Fabric (UMCF)**.

---

## 1. System Topology & Plane Separation

UMCF is split into two strictly separated planes:

### A. Control Plane (Cloudflare Workers + Durable Objects / Cloud Run Fallback)
- **Responsibilities**:
  - Cluster & node registry (`ClusterCoordinator` Durable Object / SQLite state)
  - Short-lived enrollment token issuance & verification (QR / bootstrap URI pairing)
  - Peer capability advertisement indexing (CPU/GPU/NPU capabilities, available RAM, thermal/battery state)
  - NAT traversal coordination (STUN reflection, ICE/QUIC candidate signaling, TURN relay token issuance)
  - Coarse health heartbeats and topology change broadcasts over WebSocket
- **Strict Non-Responsibilities**:
  - Never touches raw LLM weights, token activation tensors, user prompt/response text, remote desktop video frames, or raw compute payloads.

### B. Data Plane (P2P Mutually Authenticated QUIC Mesh)
- **Responsibilities**:
  - Direct peer-to-peer encrypted QUIC streams (`quinn` / `rustls` with Ed25519/X25519 mutual authentication)
  - Distributed `llama.cpp` inference pipeline execution (stage-to-stage activation tensor streaming, KV cache coordination, token streaming)
  - General-purpose sandboxed compute task distribution & result aggregation
  - Ultra-low-latency remote desktop and game streaming (video/audio datagrams & reliable input streams)
  - Content-addressed model shard transfer (BLAKE3 verified chunk streaming)

---

## 2. Canonical Workspace Structure

```text
g:\ucf\
├── crates/
│   ├── umcf-proto/             # Wire protocol schemas, envelopes, capability types, error codes
│   ├── umcf-crypto/            # Ed25519/X25519 identity, mTLS certificates, capability tokens, QR SAS
│   ├── umcf-hw-probe/          # Real CPU/SIMD, RAM, Vulkan/Metal/DX12/CUDA GPU, NNAPI/CoreML NPU probing
│   ├── umcf-resource-governor/ # Memory watermarks, thermal throttling, battery/charging policy engine
│   ├── umcf-p2p/               # QUIC endpoint, NAT traversal, hole punching, stream multiplexing
│   ├── umcf-scheduler/         # Cost-aware heterogeneous task & pipeline stage scheduler
│   ├── umcf-llama-engine/      # Safe Rust orchestration over llama.cpp FFI for local & sharded inference
│   ├── umcf-llama-sys/         # Audited FFI bindings to llama.cpp / ggml (isolated unsafe)
│   ├── umcf-stream-engine/     # Screen/audio capture, hardware encode/decode, remote input injection
│   ├── umcf-node/              # Unified node daemon uniting p2p, scheduler, inference, and telemetry
│   ├── umcf-ffi/               # flutter_rust_bridge API surface for mobile/desktop apps
│   └── umcf-cli/               # Headless CLI binary for servers, containers, and automation
├── apps/
│   ├── flutter_client/         # Multi-platform Flutter app (Android, iOS, Windows, Linux, macOS)
│   └── web_dashboard/          # Svelte 5 + TypeScript real-time mesh control & observability UI
├── services/
│   ├── control_plane_worker/   # Cloudflare Worker + Durable Objects control plane
│   └── control_plane_fallback/ # Containerized Cloud Run / self-hosted Rust/Node fallback control plane
└── podman/                     # Rootless Containerfiles, compose topologies, 30 GB quota scripts
```

---

## 3. Core Architectural Invariants

1. **Explicit Heterogeneity (No Fake Unified Memory)**:
   - Every node advertises a `NodeCapabilityProfile` containing verified local RAM, VRAM, bandwidth estimate, and link RTT.
   - The `umcf-scheduler` partitions workloads only when:
     - The model or workload exceeds single-node memory limits (requiring multi-node pipeline sharding), **or**
     - Parallel execution achieves net speedup after accounting for network RTT + serialization cost:
       $$T_{\text{distributed}} = \max_{i} (T_{\text{compute}, i}) + \sum_{i} \left(\text{RTT}_{i, i+1} + \frac{\text{Bytes}_{\text{activation}}}{B_{i, i+1}}\right) < T_{\text{local}}$$
2. **Idempotent State Transitions & Fault Recovery**:
   - Mobile nodes can disconnect at any millisecond (Wi-Fi handoff, OS sleep, battery threshold).
   - Every distributed pipeline session must maintain stage leases and support fast stage reassignment or graceful local fallback within bounded timeouts.
3. **Wire Protocol Versioning**:
   - All messages start with a 16-byte header:
     - `magic: [u8; 4]` (`b"UMCF"`)
     - `version: u16` (major/minor)
     - `msg_type: u16`
     - `flags: u32`
     - `payload_len: u32` (strictly bounded per message class)
4. **End-to-End Traceability**:
   - Propagate a 128-bit `TraceId` across control-plane signaling, QUIC stream headers, FFI calls, and UI telemetry events for deterministic debugging.
