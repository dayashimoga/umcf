---
trigger: always_on
description: Test classification taxonomy, real CPU/GPU/NPU hardware probing, heterogeneous compute honesty, and benchmarking rules for UMCF.
---

# 05 - Hardware Probing, Heterogeneity & Benchmarking Standards (UMCF)

UMCF orchestrates heterogeneous mobile, desktop, and server hardware. Engineering integrity requires strict separation between simulated topologies and real physical hardware execution, as well as honest representation of distributed physics.

---

## 1. Mandatory Five-Tier Test Classification Taxonomy

Every test suite, benchmark, and verification report in UMCF must explicitly classify each test case into one of five tiers:

1. **`[Unit]`**:
   - Deterministic, isolated tests of functions, data structures, state machines, cryptographic primitives, and parsers. Zero network or external hardware dependencies.
2. **`[Integration]`**:
   - Multi-component tests exercising real local sockets (loopback QUIC/mTLS), SQLite/storage persistence, FFI bridges, and control-plane worker handlers.
3. **`[Simulation]`**:
   - Multi-node mesh topology simulations modeling heterogeneous latency, packet loss, jitter, bandwidth caps, node churn, battery drain curves, and thermal throttling events using virtualized or parameterized execution backends.
4. **`[Hardware-Required]`**:
   - Tests that execute on real physical accelerators (Vulkan/Metal/DX12/CUDA GPUs, Android NNAPI/QNN NPUs, Apple CoreML/ANE, multi-core SIMD CPUs with `llama.cpp` GGUF inference, hardware video encoders/decoders).
   - Must dynamically detect present hardware and record exact device metadata (vendor, driver version, architecture, memory).
5. **`[Manually-Verified]`**:
   - Physical multi-device interactions requiring human observation or physical setup (e.g., scanning QR codes across two phones, verifying touch-to-desktop latency on a physical tablet, unplugging power cables to verify charging-state cutoff).

> **Strict Rule**: Never label a `[Simulation]` test as `[Hardware-Required]` or claim that a simulated NPU/GPU scheduler test certifies physical accelerator execution on unverified hardware.

---

## 2. Real Hardware Probing & Capability Discovery

1. **Authentic OS & Runtime Queries**:
   - Hardware discovery (`umcf-hw-probe`) must query real OS and runtime interfaces:
     - **CPU**: Physical/logical core counts, P-core vs E-core topology, cache sizes, clock frequencies, and SIMD feature flags (`cpuid` on x86_64, `hwcap` / sysctl on ARM64).
     - **Memory**: Total physical RAM, currently available/free RAM, swap usage, and OS memory pressure notifications.
     - **GPU**: Vulkan physical device enumeration (`vkEnumeratePhysicalDevices`), Metal device queries, DirectX DXGI adapters, or CUDA runtime device properties—recording actual dedicated VRAM vs unified shared memory.
     - **NPU / AI Accelerators**: Explicit runtime probing of Android NNAPI / Qualcomm QNN, Apple CoreML / ANE, Windows DirectML, or ONNX Runtime Execution Providers.
2. **Graceful Probe Failure & Diagnostics**:
   - If a driver or runtime library (`libvulkan`, `ggml-metal`, `QnnHtp.so`) is missing on the host system, the probe must cleanly catch the dynamic library load failure and report `BackendStatus::Unavailable { reason }` without crashing the process.

---

## 3. Honesty in Heterogeneous & Distributed Compute

1. **No Fake Unified RAM or VRAM Pooling**:
   - Network interconnects (Wi-Fi 6, 5G, Ethernet, WAN) have orders-of-magnitude higher latency and lower bandwidth than on-die memory buses.
   - Never claim or design abstractions that pretend remote node RAM acts as transparent page-swapped local memory or that two mobile GPUs act as a single zero-overhead unified VRAM device.
2. **Explicit Distributed Execution Models**:
   - Model all distributed workloads honestly:
     - **Distributed LLM Inference (`llama.cpp`)**: Layer-wise pipeline parallelism or tensor/RPC shard execution where intermediate activation tensors (~KB to MB per token step) are serialized and transmitted over QUIC streams between sequential node stages, alongside local KV-cache residency accounting.
     - **Task / Batch Compute**: Independent data-parallel jobs or DAG stages scheduled onto nodes whose local RAM/VRAM fits the assigned shard plus execution scratchpad.
     - **Remote Streaming**: Low-latency hardware-encoded video/audio frames (`H.264`/`HEVC`/`AV1` + `Opus`) and input event channels.
3. **Cost-Aware Scheduling**:
   - The UMCF scheduler must explicitly evaluate whether distributed execution across $N$ nodes is faster or more memory-feasible than local single-node execution given measured link RTT, bandwidth, and serialization overhead. If network transfer overhead exceeds compute speedup and the model fits locally, the scheduler must prefer local execution and surface the cost calculation.

---

## 4. Reproducible Benchmarking Standards

1. **Required Benchmark Metrics**:
   - Inference benchmarks must record:
     - Prompt evaluation throughput (`tokens/sec`)
     - Token generation throughput (`tokens/sec`)
     - Time-to-First-Token (`TTFT` in ms, p50 / p95 / p99)
     - Inter-token latency (`ITL` in ms, p50 / p95 / p99)
     - Network activation transfer time per token step (ms and bytes transferred)
     - Peak RSS memory and accelerator VRAM usage (MB)
     - Device thermal delta and battery discharge rate (where supported)
2. **Metadata Provenance**:
   - Every saved benchmark report must embed the exact Git commit SHA, timestamp, host OS/kernel, CPU/GPU/NPU identifiers, model GGUF filename + SHA-256 hash, quantization type (e.g., `Q4_K_M`), context length, and test classification tier.
