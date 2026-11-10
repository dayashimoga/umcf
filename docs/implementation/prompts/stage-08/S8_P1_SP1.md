# Subphase S8.P1.SP1 Execution Prompt: Multi-OS Cluster Integration Suite (Android, iOS, Windows, Linux, macOS, Web)

## 1. Role & Context
You are the Cross-Platform Distributed Integration Engineer for UMCF. In Subphase `S8.P1.SP1`, you will implement `tests/e2e-cluster/src/multi_os_suite.rs`, verifying that nodes running across **Android**, **iOS**, **Windows**, **Linux**, **macOS**, and **Web Browsers (WASM/WebRTC/WebGPU)** discover each other, authenticate via mTLS, and execute joint distributed workloads without OS-specific path separator bugs (`\` vs `/`), line-ending differences (`CRLF` vs `LF`), struct alignment/padding differences, or GPU/codec incompatibilities.

## 2. Prerequisites & Dependencies
- All Stage 1–7 crates and platform adapters built for target OS architectures.

## 3. Implementation Tasks
1. **Cross-Platform Wire & Path Normalization Audit (`tests/e2e-cluster/src/platform_compat.rs`)**:
   - Verify that all wire messages, manifest paths, content-addressed `ObjectCid` trees, and cryptographic signatures (`SignedResourceReceipt`, `ModelManifest`, `WorkloadManifest`) use canonical UTF-8 forward-slash relative virtual paths (`/`), explicit little-endian integer encoding, and UTC Unix millisecond timestamps across Windows, Linux, macOS, Android, iOS, and Web.
2. **Heterogeneous Multi-OS Cluster Test Harness (`tests/e2e-cluster/src/multi_os_suite.rs`)**:
   - Implement `MultiOsClusterHarness` capable of orchestrating both local simulated multi-OS profile nodes and physical/VM remote nodes over LAN/tailnet via a unified test controller:
     - **Scenario 1 (Cross-OS mTLS Discovery & NAT Traversal)**: Pair Windows desktop, Linux workstation, macOS laptop, Android phone, iOS tablet, and Web browser client; verify mDNS/DHT gossip convergence and direct QUIC/WebRTC connection establishment.
     - **Scenario 2 (Cross-OS Distributed WASM Map-Reduce)**: Compile a single `.wasm` module once and execute map shards simultaneously across x86_64 Windows/Linux and ARM64 macOS/Android/iOS + WebWorker WASM, verifying 100% bit-identical BLAKE3 output hashes on every OS.
     - **Scenario 3 (Cross-OS Pipeline-Parallel LLM Inference)**: Partition a GGUF model across a desktop GPU node (Layers `0..15`) and an ARM64 mobile/laptop node (Layers `16..21`), verifying greedy token parity against single-node execution.
     - **Scenario 4 (Cross-OS GPU Tile Render & Video Transcode)**: Render a frame where tiles are split across Vulkan (Windows/Linux/Android), Metal (macOS/iOS), and WebGPU (Browser) workers; transcode a multi-GOP video across mixed hardware encoders (`NVENC` + `VideoToolbox` + `MediaCodec` + software) and verify seamless stitched playback.
     - **Scenario 5 (Cross-OS Desktop-to-Mobile/Web Low-Latency Streaming)**: Stream a desktop host session simultaneously to an Android/iOS Flutter client and a Web browser client with bidirectional input and audio.
     - **Scenario 6 (Cross-OS Erasure Storage & Ledger Settlement)**: Store a file with $RS(4, 2)$ shards distributed across 6 different OS nodes, verify offline pinning on mobile, and settle dual-signed receipts in the credit ledger.

## 4. Acceptance Criteria
- >= 95% test coverage of the cross-platform integration adapter layer; 100% pass rate across all 6 multi-OS scenarios.
- Zero path-separator (`\` vs `/`), filesystem case-sensitivity (macOS/Windows case-insensitive vs Linux/Android case-sensitive), or mmap page-alignment (`4 KiB` vs `16 KiB` on Apple Silicon / Android 15+) failures across any platform.
- Identical WASM modules and integer compute shaders produce 100% bit-exact BLAKE3 outputs across `x86_64-pc-windows-msvc`, `x86_64-unknown-linux-gnu`, `aarch64-apple-darwin`, `aarch64-linux-android`, `aarch64-apple-ios`, and `wasm32-unknown-unknown`.

## 5. Required Tests
- **`[UNIT]`**:
  - `test_virtual_path_normalizer_rejects_windows_backslashes_and_drive_letters_on_wire`
  - `test_16kb_page_size_mmap_alignment_for_android15_and_apple_silicon`
  - `test_case_collision_detector_for_cross_os_storage_manifests`
- **`[INT]`**:
  - `test_scenario_1_to_6_full_multi_os_concurrent_cluster_suite`
- **`[SIM]`**:
  - Emulate heterogeneous OS capability profiles, page sizes (4K vs 16K), and clock precisions in CI across a 6-node cluster.
- **`[HW]`**:
  - Execute the Multi-OS Cluster Integration Suite across available physical/VM targets and archive per-OS telemetry receipts.
- **`[MANUAL]`**:
  - Verify live cluster topology view in Flutter & Svelte 5 displaying OS icons (Windows, Linux, macOS, Android, iOS, Web) and live cross-device job execution.

## 6. Required Evidence & Artifacts
- `artifacts/stage-08/p1/sp1_multi_os_6_scenario_report.json`
- `artifacts/stage-08/p1/sp1_cross_arch_determinism_hashes.json`

## 7. Documentation, `TODO.md` & `CHANGELOG.md` Updates
- Document cross-OS ABI/wire compatibility guarantees and 16 KiB page alignment rules in `docs/architecture/CROSS_PLATFORM_COMPATIBILITY.md`.
- Mark `S8.P1.SP1` complete in `TODO.md` and update `CHANGELOG.md`.

## 8. Rollback Criteria & Completion Gate
- **Rollback**: Any platform-specific serialization divergence, 16 KiB page alignment mmap failure, or Windows/Unix path handling bug in cross-node transfers.
- **Completion Gate**: All tests green before proceeding to `S8.P1.SP2`.
