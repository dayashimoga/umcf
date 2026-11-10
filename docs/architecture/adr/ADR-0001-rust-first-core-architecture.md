# ADR-0001: Rust-First Core Architecture with Flutter & Svelte 5 Client Shells

- **Status**: Accepted
- **Date**: 2026-11-10
- **Deciders**: UMCF Core Architecture Team
- **Related Components**: `umcf-proto`, `umcf-core`, `umcf-coordinator`, `umcf-worker`, `umcf-scheduler`, `apps/mobile-desktop`, `apps/web-dashboard`

---

## 1. Context and Problem Statement

The Universal Mobile Compute Fabric (UMCF) must execute identical distributed protocols, cryptographic handshakes, QUIC data streams, DAG scheduling algorithms, and hardware-accelerated compute engines across **Android, iOS, Windows, Linux, macOS, Web browsers, and headless Servers**. 

Maintaining separate networking, scheduling, and execution codebases in Kotlin/Swift (mobile), C#/C++ (desktop), and TypeScript/Go (server/web) would introduce severe protocol drift, memory-safety vulnerabilities in zero-copy binary parsing, and prohibitive maintenance overhead. We need a single systems programming language for the core fabric engine paired with ergonomic, native-grade UI frameworks for mobile, desktop, and web.

## 2. Decision Drivers

1. **Predictable Low Latency & Zero Garbage Collection Pauses**: Real-time 120fps game streaming and sub-millisecond inter-node tensor forwarding cannot tolerate stop-the-world GC pauses.
2. **Memory Safety on Untrusted Networks**: Parsing peer-supplied QUIC frames, CAS chunks, and tensor headers requires compile-time guarantees against buffer overflows and data races.
3. **Universal Cross-Compilation**: The core must compile natively to `x86_64`, `aarch64` (Android NDK, iOS, macOS, Linux, Windows), and `wasm32-unknown-unknown` (Web browsers).
4. **Zero-Copy C/C++ Interop**: Seamless linkage with `llama.cpp` / GGML, Vulkan/Metal/DX12 (`wgpu`), and hardware media codecs (`FFmpeg`).
5. **Single-Codebase UI Productivity**: Rapid, high-FPS reactive UI across 5 native OS targets and modern browsers.

## 3. Considered Options

| Option | Core Engine | Native Mobile/Desktop UI | Web UI | Pros | Cons |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **Option A (Selected)** | **Rust (2021/2024 Edition)** | **Flutter 3.x (`flutter_rust_bridge` v2)** | **Svelte 5 + TypeScript + Rust WASM** | Zero GC pauses; fearless concurrency; first-class QUIC (`quinn`) & WebGPU (`wgpu`); single core codebase across all 7 platforms. | Steeper compile times; FFI boundary boilerplate (mitigated by `flutter_rust_bridge` codegen). |
| **Option B** | Go (`golang`) | React Native | React 19 | Fast compilation; built-in goroutines. | Go runtime GC pauses hurt 120Hz streaming; high CGo overhead for every `llama.cpp`/GPU call; large WASM binary size (`>10 MB`). |
| **Option C** | C++20 | Qt 6 / Native Swift+Kotlin | Angular | Direct `llama.cpp` & Vulkan C++ integration. | High risk of memory corruption/CVEs in P2P parser; fragmented build systems (`CMake`/`Gradle`/`Xcode`) across 7 platforms. |

## 4. Decision Outcome

We adopt **Option A**:
- **100% of protocol, cryptography, networking, storage, scheduling, and engine orchestration logic** is written in **Rust** inside the workspace `crates/umcf-*`.
- **Native Mobile & Desktop App (`apps/mobile-desktop`)** is built with **Flutter 3.x**, binding directly to `umcf-ffi` via `flutter_rust_bridge` v2 using asynchronous streams and shared GPU texture handles.
- **Web Dashboard & Browser Worker (`apps/web-dashboard`)** is built with **Svelte 5 (Runes)** and TypeScript, loading `umcf-proto` and `umcf-core` compiled to `wasm32-unknown-unknown` alongside browser WebGPU and WebTransport APIs.

## 5. Consequences

### Positive
- **Single Source of Truth**: Wire framing (`umcf-proto`), Ed25519 security (`umcf-core`), and constraint solving (`umcf-scheduler`) are tested once and behave identically on an Android phone, a Linux server, and a Windows gaming PC.
- **Safety & Performance**: Zero-copy buffer slicing (`bytes::Bytes`) from QUIC sockets directly into memory-mapped CAS files or `llama.cpp` tensor buffers without GC pressure.

### Negative & Mitigations
- **Compile Time & Disk Usage**: Rust incremental builds can grow large. *Mitigation*: Enforce workspace-wide shared `CARGO_TARGET_DIR`, `strip = "debuginfo"` in dev/test profiles, and strict governance under ADR-0006 (30 GB Podman budget).
