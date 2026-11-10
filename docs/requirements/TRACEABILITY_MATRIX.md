# Universal Mobile Compute Fabric (UMCF) — End-to-End Traceability Matrix

> [!NOTE]
> This matrix provides complete bidirectional traceability mapping all **80 Functional Requirements (`FR-001`..`FR-080`)** and **40 Non-Functional Requirements (`NFR-001`..`NFR-040`)** to their owning **Architecture Components**, **Roadmap Stage / Phase / Subphase (`S1.P1.SP1` .. `S8.P3.SP3`)**, **ADRs**, and **Verification Test Classifications** (`Unit`, `Integration`, `Simulation`, `Hardware-Required`, `Manually Verified`).

---

## 1. Functional Requirements Traceability (`FR-001` – `FR-080`)

| Req ID | Summary | Primary Component(s) | Roadmap ID | Related ADR / Flow | Test Classification |
| :--- | :--- | :--- | :---: | :---: | :--- |
| **`FR-001`** | Ed25519 Node Identity Generation & `NodeId` Derivation | `umcf-core` | `S1.P1.SP1` | ADR-0001, Flow 1 | `Unit`, `Integration` |
| **`FR-002`** | Hardware-Backed Keystore Persistence & Zeroization | `umcf-core` | `S1.P1.SP2` | ADR-0001, Flow 1 | `Unit`, `Hardware-Required` |
| **`FR-003`** | Versioned 16-Byte Wire Framing & CRC32C Header Check | `umcf-proto` | `S1.P2.SP1` | ADR-0001 | `Unit` |
| **`FR-004`** | Canonical Ed25519 Signed `WireEnvelope<T>` | `umcf-proto`, `umcf-core` | `S1.P2.SP2` | ADR-0001, Flow 1 | `Unit`, `Integration` |
| **`FR-005`** | Time-Bounded QR Code & URI Cluster Enrollment | `umcf-core`, `apps/mobile-desktop` | `S1.P3.SP1` | Flow 1 | `Integration`, `Manually Verified` |
| **`FR-006`** | Scoped RBAC Capability Token Issuance & Verification | `umcf-core`, `umcf-coordinator` | `S1.P3.SP2` | Flow 1 | `Unit`, `Integration` |
| **`FR-007`** | Peer Revocation List & Immediate Session Termination | `umcf-core`, `umcf-coordinator` | `S1.P3.SP3` | ADR-0004 | `Integration`, `Simulation` |
| **`FR-008`** | Control Plane 64 KiB Maximum Frame Size Guard | `umcf-proto`, `services/control-plane-*` | `S1.P2.SP3` | ADR-0002 | `Unit`, `Integration` |
| **`FR-009`** | Replay Attack Timestamp & Message ID Deduplication Window | `umcf-core` | `S1.P1.SP3` | Flow 1 | `Unit` |
| **`FR-010`** | Append-Only Structured Security Audit Logging | `umcf-core`, `umcf-coordinator` | `S1.P3.SP3` | ADR-0002 | `Unit`, `Integration` |
| **`FR-011`** | Zero-Conf LAN Multicast DNS (`_umcf._udp.local`) Discovery | `umcf-core` | `S2.P1.SP1` | ADR-0004, Flow 2 | `Integration`, `Simulation` |
| **`FR-012`** | Direct P2P QUIC (`quinn`) Transport with Pinned mTLS | `umcf-core` | `S2.P1.SP2` | ADR-0004, Flow 2 | `Unit`, `Integration` |
| **`FR-013`** | Cloudflare Workers + Durable Objects Primary Control Plane | `services/control-plane-cf` | `S2.P2.SP1` | ADR-0002, Flow 2 | `Unit`, `Integration` |
| **`FR-014`** | Google Cloud Run Stateless Control Plane Failover | `services/control-plane-cloudrun`, `umcf-core` | `S2.P2.SP2` | ADR-0002 | `Integration`, `Simulation` |
| **`FR-015`** | STUN Server-Reflexive Discovery & NAT Classification | `umcf-core` | `S2.P3.SP1` | ADR-0004, Flow 2 | `Unit`, `Simulation` |
| **`FR-016`** | Simultaneous UDP Hole-Punching Across WAN NATs | `umcf-core` | `S2.P3.SP2` | ADR-0004, Flow 2 | `Simulation` |
| **`FR-017`** | Encrypted TURN / DERP-Style QUIC Relay Fallback | `umcf-core` | `S2.P3.SP3` | ADR-0004, Flow 2 | `Integration`, `Simulation` |
| **`FR-018`** | 5-Tier QUIC Stream Priority Multiplexing | `umcf-core` | `S2.P1.SP3` | ADR-0004 | `Integration`, `Simulation` |
| **`FR-019`** | Continuous Peer Link Quality Matrix Probing (RTT/BW/Loss) | `umcf-core`, `umcf-coordinator` | `S2.P2.SP3` | ADR-0004 | `Integration`, `Simulation` |
| **`FR-020`** | Seamless QUIC Connection Migration & Relay-to-Direct Upgrade | `umcf-core` | `S2.P3.SP3` | ADR-0004, Flow 2 | `Simulation` |
| **`FR-021`** | Cross-Platform CPU, SIMD & RAM Enumeration | `umcf-core`, `umcf-worker` | `S3.P1.SP1` | Flow 3 | `Unit`, `Integration` |
| **`FR-022`** | Heterogeneous GPU & Mobile NPU Backend Enumeration | `umcf-core`, `umcf-worker` | `S3.P1.SP2` | ADR-0003, Flow 3 | `Integration`, `Hardware-Required` |
| **`FR-023`** | Bounded (`<1.5s`) Hardware Calibration & Signed Attestation | `umcf-worker` | `S3.P1.SP3` | Flow 3 | `Unit`, `Integration` |
| **`FR-024`** | Mobile Memory Watermark Guard (`<= 75%` Available RAM) | `umcf-core`, `umcf-worker` | `S3.P2.SP1` | ADR-0003 | `Unit`, `Integration` |
| **`FR-025`** | 1000ms Real-Time Thermal & Battery Telemetry Polling | `umcf-core`, `umcf-worker` | `S3.P2.SP2` | Flow 6 | `Unit`, `Hardware-Required` |
| **`FR-026`** | Thermal (`>=43°C`) & Low-Battery (`<20%`) Circuit Breaker | `umcf-worker` | `S3.P2.SP3` | Flow 6 | `Unit`, `Simulation` |
| **`FR-027`** | Dynamic Master / Worker / Hybrid Role State Machine | `umcf-coordinator`, `umcf-core` | `S3.P3.SP1` | ADR-0002 | `Unit`, `Simulation` |
| **`FR-028`** | Deterministic Coordinator Election & `lease_epoch` Fencing | `umcf-coordinator` | `S3.P3.SP2` | ADR-0002 | `Unit`, `Simulation` |
| **`FR-029`** | Graceful Master Leadership Handover Before Drain/Stop | `umcf-coordinator` | `S3.P3.SP3` | ADR-0002, Flow 6 | `Integration`, `Simulation` |
| **`FR-030`** | User-Configurable Per-Device Resource Budget Sliders | `umcf-worker`, `apps/mobile-desktop` | `S3.P2.SP3` | ADR-0001 | `Unit`, `Integration` |
| **`FR-031`** | Validated Heterogeneous DAG Job Representation | `umcf-scheduler` | `S4.P1.SP1` | ADR-0003 | `Unit` |
| **`FR-032`** | Zero-Overcommit Memory & Backend Constraint Solver | `umcf-scheduler` | `S4.P1.SP2` | ADR-0003 | `Unit` |
| **`FR-033`** | Single-Node Colocation Preference Optimization Rule | `umcf-scheduler` | `S4.P1.SP3` | ADR-0003 | `Unit` |
| **`FR-034`** | Bandwidth & Latency-Aware Pipeline Stage Partitioning | `umcf-scheduler` | `S4.P2.SP1` | ADR-0003, ADR-0005 | `Unit`, `Simulation` |
| **`FR-035`** | Incremental Mid-Job Fault Rebalancing (`RebalanceDelta`) | `umcf-scheduler` | `S4.P2.SP2` | Flow 6 | `Unit`, `Simulation` |
| **`FR-036`** | 256-Bit BLAKE3 Content-Addressed Storage (CAS) | `umcf-engine-storage` | `S4.P3.SP1` | ADR-0001 | `Unit`, `Integration` |
| **`FR-037`** | 4 MiB Chunk Swarming & Multi-Peer Parallel Fetching | `umcf-engine-storage` | `S4.P3.SP2` | ADR-0004 | `Integration`, `Simulation` |
| **`FR-038`** | Zero-Copy Read-Only `mmap` Model & Artifact Access | `umcf-engine-storage`, `umcf-engine-ai` | `S4.P3.SP3` | ADR-0005 | `Unit`, `Integration` |
| **`FR-039`** | Quota-Enforced LRU CAS Eviction with Active Lease Pinning | `umcf-engine-storage` | `S4.P3.SP3` | ADR-0001 | `Unit` |
| **`FR-040`** | Resumable Interrupted Multi-Chunk CAS Transfers | `umcf-engine-storage` | `S4.P3.SP2` | ADR-0004 | `Integration`, `Simulation` |
| **`FR-041`** | GGUF Header Inspection & Tensor Byte-Offset Indexing | `umcf-engine-ai` | `S5.P1.SP1` | ADR-0005 | `Unit` |
| **`FR-042`** | Partial Transformer Layer-Range (`[start..end]`) Loading | `umcf-engine-ai` | `S5.P1.SP2` | ADR-0005, Flow 4 | `Unit`, `Integration` |
| **`FR-043`** | QUIC Stream `TensorRpcForward` Activation Pipeline | `umcf-engine-ai` | `S5.P2.SP1` | ADR-0005, Flow 4 | `Integration`, `Simulation` |
| **`FR-044`** | Adaptive Activation Quantization (`FP16` vs `Q8_0`) | `umcf-engine-ai` | `S5.P2.SP2` | ADR-0005, Flow 4 | `Unit`, `Integration` |
| **`FR-045`** | Distributed Session-Keyed KV-Cache & Prefix Reuse | `umcf-engine-ai` | `S5.P2.SP3` | ADR-0005, Flow 4 | `Unit`, `Integration` |
| **`FR-046`** | Streaming Token Events & OpenAI `/v1/chat/completions` API | `umcf-engine-ai`, `umcf-coordinator` | `S5.P3.SP1` | ADR-0005, Flow 4 | `Integration` |
| **`FR-047`** | Mid-Generation Worker Failure Recovery (KV Sync / Replay) | `umcf-engine-ai`, `umcf-scheduler` | `S5.P3.SP2` | ADR-0005, Flow 6 | `Simulation` |
| **`FR-048`** | Multi-Request Pipeline Micro-Batching Overlap | `umcf-engine-ai`, `umcf-scheduler` | `S5.P3.SP3` | ADR-0005 | `Integration`, `Simulation` |
| **`FR-049`** | Draft-Model Speculative Decoding Verification Pipeline | `umcf-engine-ai` | `S5.P3.SP3` | ADR-0005 | `Unit`, `Simulation` |
| **`FR-050`** | Multimodal Vision Encoder Stage Partitioning | `umcf-engine-ai` | `S5.P1.SP3` | ADR-0005 | `Unit`, `Integration` |
| **`FR-051`** | Cross-Platform `wgpu` Compute Shader Execution | `umcf-engine-media` | `S6.P1.SP1` | ADR-0003 | `Integration`, `Hardware-Required` |
| **`FR-052`** | `naga` WGSL Validation & Per-Dispatch Watchdog Timeout | `umcf-engine-media` | `S6.P1.SP2` | ADR-0003 | `Unit`, `Integration` |
| **`FR-053`** | Keyframe GOP-Aligned Video Splitter for Parallel Transcode | `umcf-engine-media` | `S6.P2.SP1` | ADR-0003 | `Unit`, `Integration` |
| **`FR-054`** | Hardware-Accelerated Parallel GOP Segment Transcoding | `umcf-engine-media` | `S6.P2.SP2` | ADR-0003 | `Integration`, `Hardware-Required` |
| **`FR-055`** | Frame-Accurate Media Segment Concatenation & Audio Muxing | `umcf-engine-media` | `S6.P2.SP3` | ADR-0003 | `Integration` |
| **`FR-056`** | Distributed Tile & Frame-Range 2D/3D Rendering | `umcf-engine-media` | `S6.P3.SP1` | ADR-0003 | `Unit`, `Integration` |
| **`FR-057`** | Native GPU Pipeline & Shader Binary Caching | `umcf-engine-media` | `S6.P1.SP3` | ADR-0003 | `Unit`, `Integration` |
| **`FR-058`** | Dynamic Straggler Speculative Segment Re-Dispatch | `umcf-scheduler`, `umcf-engine-media` | `S6.P3.SP2` | ADR-0003 | `Unit`, `Simulation` |
| **`FR-059`** | VAD-Segmented Parallel Audio Transcription Pipeline | `umcf-engine-media`, `umcf-engine-ai` | `S6.P3.SP3` | ADR-0003 | `Unit`, `Integration` |
| **`FR-060`** | Transcoded Segment Integrity & Frame-Count Verification | `umcf-engine-media` | `S6.P2.SP3` | ADR-0003 | `Unit`, `Integration` |
| **`FR-061`** | Zero-Copy Display & Window Surface Capture (120 FPS) | `umcf-engine-stream` | `S7.P1.SP1` | Flow 5 | `Integration`, `Hardware-Required` |
| **`FR-062`** | Ultra-Low-Latency Zero-B-Frame Hardware Video Encoding | `umcf-engine-stream` | `S7.P1.SP2` | Flow 5 | `Integration`, `Hardware-Required` |
| **`FR-063`** | QUIC Datagram / WebRTC Video Transport with Reed-Solomon FEC | `umcf-engine-stream` | `S7.P2.SP1` | ADR-0004, Flow 5 | `Unit`, `Simulation` |
| **`FR-064`** | Priority-1 Authenticated Input Return Channel Injection | `umcf-engine-stream` | `S7.P2.SP2` | Flow 5 | `Unit`, `Integration` |
| **`FR-065`** | Real-Time AIMD / GCC Adaptive Bitrate & FPS Control | `umcf-engine-stream` | `S7.P2.SP3` | Flow 5 | `Unit`, `Simulation` |
| **`FR-066`** | Low-Latency Synchronized Opus Audio Loopback Streaming | `umcf-engine-stream` | `S7.P1.SP3` | Flow 5 | `Integration` |
| **`FR-067`** | Multi-Viewer Broadcast & Per-Client Cooperative Input Control | `umcf-engine-stream` | `S7.P3.SP1` | Flow 5 | `Integration`, `Simulation` |
| **`FR-068`** | Encrypted Clipboard Sync & CAS Drag-and-Drop File Transfer | `umcf-engine-stream`, `umcf-engine-storage` | `S7.P3.SP2` | Flow 5 | `Unit`, `Integration` |
| **`FR-069`** | Metadata Hardware Cursor Pass-Through & Privacy Blanking | `umcf-engine-stream`, `apps/mobile-desktop` | `S7.P3.SP3` | Flow 5 | `Integration`, `Manually Verified` |
| **`FR-070`** | Real-Time Glass-to-Glass Latency & Telemetry HUD Overlay | `umcf-engine-stream`, `apps/*` | `S7.P3.SP3` | Flow 5 | `Unit`, `Manually Verified` |
| **`FR-071`** | Unified Flutter Mobile & Desktop Client (`apps/mobile-desktop`) | `apps/mobile-desktop`, `umcf-ffi` | `S8.P1.SP1` | ADR-0001 | `Unit`, `Integration`, `Manually Verified` |
| **`FR-072`** | Svelte 5 Web Dashboard & WebGPU Worker (`apps/web-dashboard`) | `apps/web-dashboard` | `S8.P1.SP2` | ADR-0001 | `Unit`, `Integration` |
| **`FR-073`** | Headless CLI Daemon (`umcf-cli`) with `--json` Output | `crates/umcf-cli` | `S8.P1.SP3` | ADR-0001 | `Unit`, `Integration` |
| **`FR-074`** | Interactive Cluster Mesh Radar & Live DAG Visualizer | `apps/mobile-desktop`, `apps/web-dashboard` | `S8.P2.SP1` | ADR-0001 | `Unit`, `Manually Verified` |
| **`FR-075`** | Android ForegroundService & One-Tap Pause Notification | `apps/mobile-desktop`, `umcf-worker` | `S8.P2.SP2` | Flow 6 | `Integration`, `Hardware-Required` |
| **`FR-076`** | One-Click GGUF Model & CAS Artifact Manager UI | `apps/*`, `umcf-engine-storage` | `S8.P2.SP3` | ADR-0005 | `Integration` |
| **`FR-077`** | OpenTelemetry Metrics & Prometheus `/metrics` Exporter | `umcf-coordinator`, `umcf-worker` | `S8.P3.SP1` | ADR-0001 | `Unit`, `Integration` |
| **`FR-078`** | Multi-Node Podman Chaos & Fault-Injection Simulation Suite | `crates/umcf-cli`, `containers/*` | `S8.P3.SP2` | ADR-0006 | `Simulation` |
| **`FR-079`** | Zero-Downtime Dynamic Configuration Reload (`SIGHUP`/API) | `umcf-worker`, `umcf-coordinator` | `S8.P3.SP1` | ADR-0001 | `Integration` |
| **`FR-080`** | Redacted Diagnostic Support Bundle Exporter | `umcf-core`, `crates/umcf-cli` | `S8.P3.SP3` | ADR-0001 | `Unit`, `Integration` |

---

## 2. Non-Functional Requirements Traceability (`NFR-001` – `NFR-040`)

| Req ID | Category & Target SLO | Primary Component(s) | Roadmap ID | Related ADR | Test Classification |
| :--- | :--- | :--- | :---: | :---: | :--- |
| **`NFR-001`** | LAN mDNS Discovery `P95 <= 2,000 ms` | `umcf-core` | `S2.P1.SP1` | ADR-0004 | `Simulation` |
| **`NFR-002`** | WAN Hole-Punch `<= 600 ms`, Relay Fallback `<= 900 ms` | `umcf-core` | `S2.P3.SP2` | ADR-0004 | `Simulation` |
| **`NFR-003`** | Inter-Node Activation Framing Overhead `<= 2.5 ms/hop` | `umcf-engine-ai`, `umcf-proto` | `S5.P2.SP1` | ADR-0005 | `Integration`, `Simulation` |
| **`NFR-004`** | Remote Stream Glass-to-Glass LAN Median `<= 16.6 ms` | `umcf-engine-stream` | `S7.P1.SP2` | ADR-0004 | `Simulation`, `Hardware-Required` |
| **`NFR-005`** | Remote Input Return Channel LAN `P99 <= 3.0 ms` | `umcf-engine-stream` | `S7.P2.SP2` | ADR-0004 | `Integration` |
| **`NFR-006`** | Scheduler Placement Solver `<= 100 ms` (Rebalance `<= 50 ms`) | `umcf-scheduler` | `S4.P1.SP2` | ADR-0003 | `Unit` |
| **`NFR-007`** | CAS Multi-Chunk Swarm `>= 80%` LAN Link Saturation | `umcf-engine-storage` | `S4.P3.SP2` | ADR-0004 | `Integration`, `Simulation` |
| **`NFR-008`** | Pipeline Inference Efficiency `>= 75%` / Batch `>= 1.8x` | `umcf-engine-ai` | `S5.P3.SP3` | ADR-0005 | `Simulation` |
| **`NFR-009`** | Single Coordinator Scalability `>= 64` Concurrent Nodes | `umcf-coordinator` | `S3.P3.SP1` | ADR-0002 | `Simulation` |
| **`NFR-010`** | Wire Framing Throughput `>= 100k msg/s` or `>= 4 GB/s` | `umcf-proto` | `S1.P2.SP1` | ADR-0001 | `Unit` |
| **`NFR-011`** | Concurrent QUIC Streams `>= 256` per Peer Connection | `umcf-core` | `S2.P1.SP3` | ADR-0004 | `Integration` |
| **`NFR-012`** | Strict `<= 30.0 GB` Podman + Build Cache Disk Footprint Cap | `containers/*`, `scripts/*` | `S1.P1.SP1` | ADR-0006 | `Integration`, `Simulation` |
| **`NFR-013`** | Idle Daemon Memory Overhead `<= 35 MiB` (`<= 45 MiB` Mobile) | `umcf-worker`, `umcf-coordinator` | `S8.P1.SP3` | ADR-0001 | `Integration` |
| **`NFR-014`** | Mobile OOM Guard (`<= 75%` Available RAM Ceiling) | `umcf-core`, `umcf-worker` | `S3.P2.SP1` | ADR-0003 | `Unit` |
| **`NFR-015`** | Browser WASM32 Linear Memory Ceiling `<= 2.0 GiB` | `apps/web-dashboard` | `S8.P1.SP2` | ADR-0001 | `Integration` |
| **`NFR-016`** | Stripped Release Binary Size `<= 25 MiB` per Target | `crates/umcf-cli`, `umcf-ffi` | `S8.P1.SP3` | ADR-0006 | `Integration` |
| **`NFR-017`** | Zero Unbounded Channels (`0` `mpsc::unbounded_channel`) | All Rust Crates | `S1.P2.SP3` | ADR-0001 | `Unit` |
| **`NFR-018`** | Mandatory Thermal Throttling (`41°C`) & Cutoff (`43°C`) | `umcf-worker` | `S3.P2.SP3` | ADR-0003 | `Unit`, `Simulation` |
| **`NFR-019`** | Thermal Hysteresis Recovery (`<= 39°C` & `>= 60s`) | `umcf-worker` | `S3.P2.SP3` | ADR-0003 | `Unit` |
| **`NFR-020`** | Unplugged Battery Floor (`< 20%` Drain, `>= 30%` Resume) | `umcf-worker` | `S3.P2.SP3` | ADR-0003 | `Unit` |
| **`NFR-021`** | Idle Standby CPU `< 1.0%` & Battery Drain `< 1.5%/hr` | `umcf-worker` | `S8.P2.SP2` | ADR-0001 | `Simulation`, `Hardware-Required` |
| **`NFR-022`** | GPU Dispatch Chunking `<= 500 ms` (No OS TDR Resets) | `umcf-engine-media`, `umcf-engine-ai` | `S6.P1.SP2` | ADR-0003 | `Unit`, `Integration` |
| **`NFR-023`** | Modern Crypto Only (Ed25519, TLS 1.3, BLAKE3) | `umcf-core`, `umcf-proto` | `S1.P1.SP1` | ADR-0001 | `Unit` |
| **`NFR-024`** | Zero-Knowledge Control Plane & End-to-End Encrypted Relays | `umcf-core`, `services/*` | `S2.P3.SP3` | ADR-0002, ADR-0004 | `Integration`, `Simulation` |
| **`NFR-025`** | 100% Pre-Use BLAKE3 Leaf Hash Verification on CAS Chunks | `umcf-engine-storage` | `S4.P3.SP1` | ADR-0001 | `Unit` |
| **`NFR-026`** | `#[forbid(unsafe_code)]` on Protocol, Core, Scheduler, Coord | `umcf-proto`, `umcf-scheduler`, `umcf-coordinator` | `S1.P1.SP1` | ADR-0001 | `Unit` |
| **`NFR-027`** | Redacted `Debug` & `ZeroizeOnDrop` for Keys/Tokens | `umcf-core` | `S1.P1.SP2` | ADR-0001 | `Unit` |
| **`NFR-028`** | Sandboxed Execution (No Arbitrary Native Binary Execution) | `umcf-worker` | `S3.P2.SP1` | ADR-0001 | `Unit` |
| **`NFR-029`** | 100% Offline Air-Gapped LAN Operation | `umcf-coordinator`, `umcf-core` | `S2.P1.SP1` | ADR-0002 | `Simulation` |
| **`NFR-030`** | Mid-Task Worker Crash Recovery `<= 5,000 ms` (`<= 1.5s` Preempt) | `umcf-scheduler`, `umcf-coordinator` | `S5.P3.SP2` | ADR-0005 | `Simulation` |
| **`NFR-031`** | Coordinator Failover `<= 5,000 ms` with `lease_epoch` Fencing | `umcf-coordinator` | `S3.P3.SP2` | ADR-0002 | `Simulation` |
| **`NFR-032`** | Zero Panics on 100,000 Fuzz/Property-Based Malformed Inputs | `umcf-proto`, `umcf-engine-*` | `S8.P3.SP2` | ADR-0001 | `Unit` |
| **`NFR-033`** | Atomic Config & Index Persistence (Temp + Fsync + Rename) | `umcf-core`, `umcf-engine-storage` | `S4.P3.SP1` | ADR-0001 | `Unit` |
| **`NFR-034`** | Cross-Node Numerical Parity (`cosine_sim >= 0.999`) | `umcf-engine-ai`, `umcf-engine-media` | `S5.P2.SP1` | ADR-0005 | `Integration`, `Simulation` |
| **`NFR-035`** | `< 60s` Zero-to-Cluster QR Onboarding UX | `apps/mobile-desktop`, `umcf-core` | `S8.P1.SP1` | ADR-0001 | `Manually Verified` |
| **`NFR-036`** | WCAG 2.1 AA Accessibility (Semantics, ARIA, Contrast) | `apps/mobile-desktop`, `apps/web-dashboard` | `S8.P2.SP1` | ADR-0001 | `Unit`, `Manually Verified` |
| **`NFR-037`** | Non-Blocking 60/120 FPS UI Thread (`< 16.6 ms` Frame Time) | `apps/mobile-desktop`, `apps/web-dashboard` | `S8.P1.SP1` | ADR-0001 | `Integration` |
| **`NFR-038`** | Zero Compiler & Linter Warnings (`clippy -D warnings`) | Entire Workspace | `S1.P1.SP1` | ADR-0001 | `Unit` |
| **`NFR-039`** | `>= 95.0%` Automated Line Test Coverage Across Crates/Services | Entire Workspace | `S8.P3.SP3` | ADR-0006 | `Unit`, `Integration`, `Simulation` |
| **`NFR-040`** | Reproducible Multi-Platform Build Matrix | Entire Workspace | `S8.P3.SP3` | ADR-0006 | `Integration` |
