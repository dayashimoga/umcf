---
description: Workflow for certifying real CPU, GPU, and NPU hardware discovery, llama.cpp inference benchmarking, thermal throttling, and physical execution in UMCF.
---

# UMCF Hardware Certification & Benchmarking Workflow

Execute this workflow whenever verifying `[Hardware-Required]` capabilities on physical host machines or mobile devices, certifying CPU/GPU/NPU acceleration, or generating official UMCF hardware benchmark reports.

---

## Step 1: Host Hardware Discovery & Probe Verification

1. Run the UMCF hardware discovery diagnostic command:
   ```powershell
   cargo run --release -p umcf-cli -- hw-probe --json
   ```
2. Verify that the output accurately records real OS/hardware metrics:
   - **CPU**: Architecture (`x86_64` / `aarch64`), physical and logical core counts, SIMD instruction sets (`AVX2`, `AVX512`, `FMA`, `NEON`, `FP16`, `DOTPROD`), and clock speed.
   - **Memory**: Total physical RAM (MB), currently available RAM (MB), and safety watermark allocation limit.
   - **GPU Backends**: Enumerated Vulkan, Metal, DirectX 12, or CUDA adapters with real vendor/device names, driver versions, and dedicated vs shared VRAM sizes.
   - **NPU / AI Accelerators**: Probed status of NNAPI, QNN, CoreML/ANE, DirectML, or ONNX Runtime Execution Providers—explicitly reporting `Available` (with device metadata) or `Unavailable { reason }`.
3. **Honesty Check**:
   - Confirm that any backend missing hardware or driver support on the current machine is explicitly marked `Unavailable` and **never** reported as active or accelerated.

---

## Step 2: Local & Distributed `llama.cpp` Hardware Inference Certification

1. **Verify GGUF Model Integrity**:
   - Verify the test GGUF model's SHA-256 / BLAKE3 digest and inspect its layer count, context length, and quantization type (e.g., `TinyLlama-1.1B-Chat-v1.0.Q4_K_M.gguf` or `Qwen2.5-0.5B-Instruct-Q4_K_M.gguf`).
2. **Execute Single-Node Hardware Benchmark (`[Hardware-Required]`)**:
   - Run real prompt evaluation and token generation across each available backend on the host (CPU SIMD, and GPU Vulkan/Metal/CUDA if present):
     ```powershell
     cargo run --release -p umcf-cli -- bench-inference --model <path-to-gguf> --prompt-tokens 128 --gen-tokens 64 --json
     ```
   - Record:
     - Active hardware backend verified (`CPU_AVX2`, `GPU_VULKAN`, `GPU_METAL`, etc.)
     - Time-to-First-Token (`TTFT` ms)
     - Prompt evaluation speed (`tokens/sec`)
     - Generation throughput (`tokens/sec`)
     - Peak RSS RAM and VRAM usage (MB)
3. **Execute Multi-Node Pipeline Hardware Benchmark (`[Hardware-Required]`)**:
   - Start two or more local/LAN UMCF node instances bound to real QUIC UDP ports, partition model layers across the nodes, and run end-to-end distributed generation.
   - Record stage-to-stage QUIC activation transfer latency (ms/token), bytes transmitted per token step, and verify output logits/tokens match deterministic reference generation.

---

## Step 3: Resource Governor, Thermal & Memory Pressure Certification

1. **Memory Watermark Enforcement**:
   - Configure a strict memory cap via `umcf-resource-governor` below a model's required footprint and verify the engine deterministically rejects local full-model loading with `InferenceError::InsufficientMemory` and triggers multi-node sharding or quantization fallback instead of crashing.
2. **Thermal & Battery Preemption**:
   - Inject or trigger a thermal throttling warning (`ThermalState::Serious`) or battery floor breach during active generation and verify that workload preemption / stage drain completes within the target SLO (< 500 ms for local stop, seamless handoff for pipeline drain).

---

## Step 4: Publish Hardware Certification Report

Write a structured JSON and Markdown certification report (`docs/benchmarks/hw-certification-<host>-<date>.md`) containing:
- Exact Git commit SHA and timestamp
- Host OS, kernel version, CPU model, GPU model + driver, NPU status, and physical RAM
- Explicit test classification tags (`[Hardware-Required]` vs `[Simulation]`)
- Full benchmark tables (`TTFT`, `ITL`, `tokens/sec`, memory footprint, network overhead)
- Verification signatures / log excerpts proving real hardware execution
