# Universal Mobile Compute Fabric (UMCF) — Non-Functional Requirements Specification

> [!IMPORTANT]
> This document defines the **40 normative Non-Functional Requirements (`NFR-001` through `NFR-040`)** governing latency, throughput, memory bounds, the strict 30 GB Podman storage ceiling, mobile thermal/battery safety, zero-trust security, offline-first resilience, UI accessibility, and `>= 95%` automated test coverage.

---

## 1. Latency & Real-Time Responsiveness (`NFR-001` – `NFR-006`)

| Req ID | Category | Requirement Statement | Quantitative Target / SLO | Verification Method |
| :--- | :--- | :--- | :--- | :--- |
| **`NFR-001`** | Latency | **LAN Peer Discovery Latency**: Two UMCF nodes joining the same local broadcast subnet SHALL discover and authenticate each other via mDNS + QUIC handshake. | `P95 <= 2,000 ms` from interface up | Automated Multi-Container Podman LAN Test |
| **`NFR-002`** | Latency | **WAN Direct Hole-Punching & Relay Failover Time**: Establishing a WAN P2P QUIC session across Cone NATs or failing over to an encrypted TURN/DERP relay on Symmetric NATs SHALL be bounded. | Direct: `P95 <= 600 ms`; Relay Fallback: `P95 <= 900 ms` | Podman `nftables` NAT Topology Simulation |
| **`NFR-003`** | Latency | **Inter-Node Pipeline Activation Overhead**: Framing, signing/encrypting, and transmitting a single decode-step hidden state (`FP16` or `Q8_0`, `dim=4096`) over a Gigabit/Wi-Fi 6 LAN hop SHALL add minimal overhead. | `P95 <= 2.5 ms` per pipeline hop above raw wire RTT | Benchmark Spike `P-01` & Integration Suite |
| **`NFR-004`** | Latency | **Remote Streaming Glass-to-Glass Latency (LAN)**: End-to-end frame capture, hardware encode, QUIC datagram transport, hardware decode, and surface presentation at 1080p60/120 on LAN. | Median `<= 16.6 ms`, `P99 <= 25.0 ms` | Synthetic Timestamped Frame Loopback Test |
| **`NFR-005`** | Latency | **Remote Input Return Channel Latency**: High-priority mouse, keyboard, and gamepad events transmitted from viewer to host OS input queue on LAN. | `P99 <= 3.0 ms` | Integration Benchmark (`umcf-engine-stream`) |
| **`NFR-006`** | Latency | **DAG Scheduler Placement Solver Time**: Computing an optimal DAG placement or incremental fault rebalance across clusters of up to 32 heterogeneous nodes. | Initial Placement: `<= 100 ms`; Incremental Rebalance: `<= 50 ms` | Criterion Micro-Benchmark (`umcf-scheduler`) |

---

## 2. Throughput & Scalability (`NFR-007` – `NFR-011`)

| Req ID | Category | Requirement Statement | Quantitative Target / SLO | Verification Method |
| :--- | :--- | :--- | :--- | :--- |
| **`NFR-007`** | Throughput | **CAS Swarm Transfer Saturation**: Multi-chunk BLAKE3 content-addressed transfers over QUIC SHALL saturate available LAN bandwidth without blocking high-priority control/tensor streams. | `>= 80%` of raw iperf3 UDP link capacity on LAN | Integration Bandwidth Benchmark (`umcf-engine-storage`) |
| **`NFR-008`** | Throughput | **Pipeline Parallel Inference Efficiency**: Partitioning an LLM across $k \in \{2, 3, 4\}$ nodes when the model exceeds single-node memory SHALL achieve high pipeline efficiency vs. theoretical compute speed. | Decode throughput `>= 75%` of ideal compute-bound rate on LAN; Multi-request batch throughput `>= 1.8x` single-request | Multi-Node Inference Benchmark (`umcf-engine-ai`) |
| **`NFR-009`** | Scalability | **Cluster Node Capacity (Single Coordinator)**: A single `Local Autonomous Coordinator` or Cloudflare Durable Object instance SHALL support concurrent active nodes sending `1,000ms` heartbeats. | Up to `64` active nodes per cluster (`<= 5%` coordinator CPU overhead) | Load Simulation Test (`umcf-coordinator`) |
| **`NFR-010`** | Throughput | **Wire Protocol Framing Throughput**: Serializing, CRC32C-checking, and parsing `WireEnvelope` frames on a single CPU core. | `>= 100,000` control messages/sec or `>= 4.0 GB/s` tensor framing per core | Criterion Benchmark (`umcf-proto`) |
| **`NFR-011`** | Scalability | **Concurrent QUIC Streams per Peer Connection**: Each peer-to-peer QUIC connection SHALL support concurrent active bidirectional and unidirectional streams without degradation. | `>= 256` concurrent streams per peer pair | Transport Stress Test (`umcf-core`) |

---

## 3. Memory, Binary Footprint & 30 GB Podman Cap (`NFR-012` – `NFR-017`)

| Req ID | Category | Requirement Statement | Quantitative Target / SLO | Verification Method |
| :--- | :--- | :--- | :--- | :--- |
| **`NFR-012`** | Disk Cap | **Strict 30 GB Podman & Build Footprint Ceiling**: Total disk consumption of all Podman container images, overlay volumes, Rust `target/` directories, and build caches SHALL never exceed 30 GB. | Hard Cap: `<= 30.0 GB` (Automated fail-safe alert at `25.0 GB`) | Automated `scripts/check-disk-budget` Gate |
| **`NFR-013`** | Memory | **Idle Daemon RAM Overhead**: The base memory footprint (RSS) of `umcf-worker` + `umcf-coordinator` when idle (no active model loaded in RAM) SHALL be minimal. | `<= 35 MiB` RSS on Linux/Windows/macOS; `<= 45 MiB` on Android/iOS | Process RSS Telemetry Assertion |
| **`NFR-014`** | Memory | **Mobile OOM Prevention Ceiling**: On Android and iOS devices, UMCF SHALL never allocate or `mmap`-pin more than 75% of currently available free OS RAM. | Total UMCF allocation `<= 0.75 * available_ram_bytes` at all times | Unit & Fault-Injection Memory Guard Tests |
| **`NFR-015`** | Memory | **WASM32 Browser Worker Memory Bound**: When compiled to `wasm32-unknown-unknown` in `apps/web-dashboard`, memory usage SHALL remain within browser WebAssembly limits. | `<= 2.0 GiB` linear WASM memory (`<= 4.0 GiB` hard ceiling) | Browser WASM Integration Test |
| **`NFR-016`** | Footprint | **Stripped Release Binary Size**: The stripped release binary of the unified headless CLI/daemon (`umcf-cli`) and mobile FFI shared library (`libumcf_ffi.so`). | `<= 25 MiB` stripped binary size per target architecture | Release Build Artifact Inspection |
| **`NFR-017`** | Memory | **Zero Unbounded Channels or Queues**: Every asynchronous channel (`tokio::sync::mpsc`) and network buffer pool in the codebase SHALL have an explicit bounded capacity with backpressure. | `0` instances of `mpsc::unbounded_channel` in production crates | Static Lint / AST Check in CI |

---

## 4. Mobile Battery, Thermal & Hardware Safety (`NFR-018` – `NFR-022`)

| Req ID | Category | Requirement Statement | Quantitative Target / SLO | Verification Method |
| :--- | :--- | :--- | :--- | :--- |
| **`NFR-018`** | Thermal Safety | **Mandatory Thermal Throttling & Cutoff Thresholds**: Mobile workers SHALL automatically scale down concurrency at `41.0°C` battery (`Serious`) and immediately preempt/drain all tasks at `43.0°C` battery or `85.0°C` die temperature (`Critical`). | Task drain initiated within `<= 500 ms` of crossing `43.0°C` threshold | Simulated Thermal Sensor Ramp Test |
| **`NFR-019`** | Thermal Safety | **Cool-Down Hysteresis Enforcement**: Once a node enters `ThrottledStandby` due to critical temperature, it SHALL NOT re-admit compute leases until temperature drops below the hysteresis recovery threshold. | Recovery requires `temp <= 39.0°C` AND `>= 60 seconds` minimum cool-down | State Machine Unit & Simulation Test |
| **`NFR-020`** | Battery Safety | **Unplugged Battery Preservation Floor**: By default, mobile workers running on battery power SHALL refuse new compute leases and drain active tasks when battery charge falls below 20%. | Cutoff at `< 20%` unplugged; resume only at `>= 30%` or AC plugged in | Power State Policy Unit Tests |
| **`NFR-021`** | Battery Safety | **Idle Background Battery Drain**: When enrolled in a cluster in `Idle / Standby` worker mode (waiting for jobs), background CPU and network activity SHALL have negligible battery impact. | `< 1.0%` CPU average utilization; `< 1.5%` battery drain per hour | 60-Second Idle Wakeup Counter Profiling |
| **`NFR-022`** | GPU Safety | **GPU Watchdog & TDR Prevention**: Compute shader dispatches (`wgpu`) and `llama.cpp` GPU batches SHALL be chunked so no single GPU kernel launch exceeds OS Timeout Detection and Recovery (TDR) limits (`2,000ms` on Windows/Android). | Individual GPU kernel dispatch time `<= 500 ms` | GPU Dispatch Chunking Validation |

---

## 5. Security, Cryptography & Privacy (`NFR-023` – `NFR-028`)

| Req ID | Category | Requirement Statement | Quantitative Target / SLO | Verification Method |
| :--- | :--- | :--- | :--- | :--- |
| **`NFR-023`** | Cryptography | **Mandatory Modern Cryptographic Primitives**: All identity, signing, hashing, and transport encryption SHALL use exclusively **Ed25519** (RFC 8032), **X25519**, **TLS 1.3** (`rustls`), **ChaCha20-Poly1305 / AES-256-GCM**, and **BLAKE3**. | `0` usage of MD5, SHA-1, RSA-<2048, or unencrypted sockets | Dependency & Crypto Audit (`cargo tree`) |
| **`NFR-024`** | Privacy | **Zero-Knowledge Control Plane & Relays**: Neither the WAN Control Plane (Cloudflare / Cloud Run) nor TURN/DERP relay nodes SHALL ever have access to plaintext user prompts, model weights, tensor activations, or video streams. | 100% of Data Plane payloads encrypted end-to-end via peer-pinned mTLS | Packet Capture & Relay Inspection Test |
| **`NFR-025`** | Integrity | **100% Pre-Use BLAKE3 Chunk Verification**: Every `4 MiB` CAS chunk received from a peer SHALL be cryptographically verified against its expected BLAKE3 hash before being written to persistent storage or mapped into an engine. | 100% detection and rejection of single-bit flips in transit | Fault-Injection Bit-Flip Fuzz Test |
| **`NFR-026`** | Memory Safety | **Unsafe Rust Isolation & Justification**: Use of `unsafe` blocks SHALL be strictly prohibited in `umcf-proto`, `umcf-scheduler`, and `umcf-coordinator`, and isolated only to audited FFI/mmap boundaries (`umcf-engine-*`, `umcf-ffi`) with `// SAFETY:` comments. | `#[forbid(unsafe_code)]` enforced on pure-Rust protocol/scheduler crates | Compiler Attribute & Clippy Lint Gate |
| **`NFR-027`** | Secret Hygiene | **Zero Secret Leakage in Logs or Crash Dumps**: Private keys, enrollment nonces, and RBAC JWT tokens SHALL implement redacted `Debug`/`Display` traits and `ZeroizeOnDrop`. | Automated regex scan over `TRACE`-level logs finds `0` raw key/token strings | Log Redaction Integration Test |
| **`NFR-028`** | Sandboxing | **No Arbitrary Native Code Execution**: Workers SHALL never download and execute arbitrary untrusted native binaries (`.exe`, `.so`, shell scripts) from peers; only parameterized tasks on built-in engines and sandboxed WGSL/WASM are permitted. | Task specification schema structurally disallows arbitrary OS command strings | Schema & Security Boundary Audit |

---

## 6. Reliability, Fault Tolerance & Offline-First Resilience (`NFR-029` – `NFR-034`)

| Req ID | Category | Requirement Statement | Quantitative Target / SLO | Verification Method |
| :--- | :--- | :--- | :--- | :--- |
| **`NFR-029`** | Resilience | **100% Offline LAN Operation**: A cluster of nodes on a local Wi-Fi/Ethernet network with zero WAN/internet connectivity SHALL support full discovery, coordinator election, CAS transfer, distributed AI inference, and streaming. | 100% of core workflows pass in an air-gapped (`--network isolated`) Podman bridge | Air-Gapped LAN Integration Suite |
| **`NFR-030`** | Fault Recovery | **Worker Mid-Task Failover Recovery Time**: When an active worker in a multi-node inference or media job abruptly crashes (`SIGKILL`) or disconnects, the cluster SHALL detect the failure and resume execution on a surviving node. | Failure Detection + Rebalance + Resume `<= 5,000 ms` on LAN (`<= 1,500 ms` on graceful preempt) | Chaos Kill-9 Recovery Test |
| **`NFR-031`** | Consensus | **Coordinator Failover & Split-Brain Immunity**: If the active Master node crashes or partitions, surviving nodes SHALL elect a new Master and fence off stale masters using monotonic `lease_epoch` counters. | Coordinator failover `<= 5,000 ms`; `0` split-brain double-executions | Network Partition Simulation Test |
| **`NFR-032`** | Stability | **Crash-Free Fuzzing Resilience**: All network wire parsers (`umcf-proto`), GGUF header parsers, and WGSL validators SHALL never panic (`unwrap`/`expect` forbidden on untrusted inputs) when fed arbitrary malformed byte streams. | `0` panics across `100,000` randomized proptest/fuzz iterations | Property-Based & Fuzz Test Suite |
| **`NFR-033`** | Durability | **Atomic Local State Persistence**: Node identity certificates, CAS indices, and cluster configuration files SHALL be written atomically (write-to-temp + `fsync` + atomic rename) to survive sudden power loss. | Simulated mid-write crash leaves previous valid configuration intact | Atomic File Persistence Unit Test |
| **`NFR-034`** | Numerical | **Cross-Backend Numerical Determinism**: Distributed LLM layer pipelines across mixed CPU/GPU architectures SHALL produce logits numerically consistent with single-node execution. | Top-1 token agreement `>= 98%` (greedy `temp=0.0`); hidden state cosine similarity `>= 0.999` | Cross-Node Numerical Parity Test |

---

## 7. Usability, Accessibility, Maintainability & Testing (`NFR-035` – `NFR-040`)

| Req ID | Category | Requirement Statement | Quantitative Target / SLO | Verification Method |
| :--- | :--- | :--- | :--- | :--- |
| **`NFR-035`** | Usability | **60-Second Zero-to-Cluster Onboarding**: A user installing UMCF on two devices SHALL be able to initialize a cluster, scan the enrollment QR code, and see both devices connected in the mesh radar in under 60 seconds. | `<= 3` user interactions; `< 60 seconds` total onboarding time | End-to-End UX Flow Verification |
| **`NFR-036`** | Accessibility | **WCAG 2.1 AA UI Compliance**: Both the Flutter (`apps/mobile-desktop`) and Svelte 5 (`apps/web-dashboard`) applications SHALL support screen readers (Semantics/ARIA), full keyboard navigation, and `>= 4.5:1` contrast ratios. | 100% pass rate on automated accessibility audits (`axe-core` / Flutter Semantics checks) | Automated UI Accessibility Test |
| **`NFR-037`** | UI Performance | **60/120 FPS Non-Blocking UI Rendering**: All cryptography, networking, CAS hashing, and compute orchestration SHALL execute on background Rust worker threads, never blocking the Flutter/browser main UI thread. | `< 16.6 ms` UI frame build/raster time during active background compute | Flutter / Browser Frame Timing Assertion |
| **`NFR-038`** | Code Quality | **Zero Compiler Warnings & Strict Clippy Gate**: All Rust crates, TypeScript services, and Dart packages SHALL compile with zero warnings under strict linter profiles (`cargo clippy -- -D warnings`, `eslint`, `flutter analyze`). | `0` warnings or errors across the entire repository | CI Static Analysis Gate |
| **`NFR-039`** | Test Coverage | **Comprehensive Automated Test Coverage (`>= 95%`)**: Every core crate and service SHALL maintain high line and branch test coverage across unit, property-based, and containerized integration tests. | `>= 95.0%` aggregate line coverage across `crates/*` and `services/*` | `cargo llvm-cov` / `vitest` Coverage Gate |
| **`NFR-040`** | Portability | **Cross-Platform Build Reproducibility**: The codebase SHALL compile cleanly across Linux, Windows, macOS, Android, iOS, and WebAssembly using documented build scripts without proprietary commercial SDK dependencies. | 100% build matrix pass rate | Multi-Target Build Verification |
