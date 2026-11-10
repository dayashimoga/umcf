# Stage 05 Execution Prompt: Real Distributed AI Inference, Model Management & Private AI

## 1. Role & Context
You are the Principal AI Systems Architect & Distributed Inference Lead for the **Unified Mobile-First Compute Fabric (UMCF)**. Stage 5 implements the complete, zero-mock distributed AI inference engine, content-addressed GGUF model registry, safe Rust FFI bindings to `llama.cpp`/GGML, heterogeneous hardware backend dispatch (CPU AVX2/AVX-512/NEON, Vulkan, Metal, CUDA/ROCm), memory-aware pipeline/tensor layer partitioning across mobile and desktop nodes, and the Private AI local enclave with an OpenAI-compatible local API.

Every subphase in Stage 5 follows the strict UMCF hierarchy:
`Stage → Phase → Subphase → Implementation tasks → Acceptance criteria → Tests → Evidence → Documentation`.

## 2. Stage 05 Phase & Subphase Index

| Phase | Subphase | Prompt File | Title & Scope |
| :--- | :--- | :--- | :--- |
| **S5.P1** | — | `docs/implementation/prompts/stage-05/S5_P1_PROMPT.md` | **Model Registry, Content-Addressed GGUF Management & Quantization Verification** |
| S5.P1 | S5.P1.SP1 | `docs/implementation/prompts/stage-05/S5_P1_SP1.md` | Content-Addressed Model Store & Resumable P2P Shard Distribution |
| S5.P1 | S5.P1.SP2 | `docs/implementation/prompts/stage-05/S5_P1_SP2.md` | GGUF Metadata Inspector, Memory Footprint Estimator & Compatibility Validator |
| S5.P1 | S5.P1.SP3 | `docs/implementation/prompts/stage-05/S5_P1_SP3.md` | Model Integrity Verification, Licensing & Access Control |
| **S5.P2** | — | `docs/implementation/prompts/stage-05/S5_P2_PROMPT.md` | **Native `llama.cpp` Engine Integration & Heterogeneous Backend Dispatch** |
| S5.P2 | S5.P2.SP1 | `docs/implementation/prompts/stage-05/S5_P2_SP1.md` | Safe Rust FFI Bindings to `llama.cpp` & GGML Backend Loader |
| S5.P2 | S5.P2.SP2 | `docs/implementation/prompts/stage-05/S5_P2_SP2.md` | Hardware Backend Selection (CPU AVX/NEON, Vulkan, Metal, CUDA/ROCm) with Fallback |
| S5.P2 | S5.P2.SP3 | `docs/implementation/prompts/stage-05/S5_P2_SP3.md` | Streaming Token Generation, KV-Cache Management & Cancellation |
| **S5.P3** | — | `docs/implementation/prompts/stage-05/S5_P3_PROMPT.md` | **Distributed Pipeline/Tensor Inference & Private AI Privacy Enclave** |
| S5.P3 | S5.P3.SP1 | `docs/implementation/prompts/stage-05/S5_P3_SP1.md` | Memory-Aware Layer Partitioning Across Heterogeneous Nodes |
| S5.P3 | S5.P3.SP2 | `docs/implementation/prompts/stage-05/S5_P3_SP2.md` | Distributed `llama.cpp` RPC Activation Transfer & Fault Recovery |
| S5.P3 | S5.P3.SP3 | `docs/implementation/prompts/stage-05/S5_P3_SP3.md` | Private AI Local-Only Mode, Prompt Encryption & OpenAI-Compatible Local API |

## 3. Prerequisites & Dependencies
- **Stage 1–4 Completion Gate**: Cryptographic node identity (`umcf-crypto`), QUIC/WebRTC P2P transport (`umcf-net`), capability profiler & thermal telemetry (`umcf-hw`), and distributed DAG scheduler (`umcf-scheduler`) must pass 100% of Stage 4 gate tests.
- **Native Toolchains**: CMake >= 3.24, Clang/LLVM >= 17, Vulkan SDK >= 1.3.275, Xcode Metal toolchain (macOS/iOS), Android NDK r26d (Android arm64-v8a), and optional CUDA/ROCm SDKs.
- **Zero-Placeholder Policy**: No mock token generators, dummy GGUF parsers, or simulated inference loops are allowed in production crates (`crates/umcf-ai`, `crates/umcf-gguf`, `crates/umcf-llama-sys`).

## 4. Implementation Tasks (Stage-Level)
1. **Workspace Crates**:
   - Create and wire `crates/umcf-gguf` (zero-copy GGUF v2/v3 parser, BLAKE3 shard verifier, memory estimator, license & RBAC policy engine).
   - Create and wire `crates/umcf-llama-sys` and `crates/umcf-ai` (safe RAII wrappers over `llama.cpp`, GGML backend loader, KV-cache paged manager, distributed RPC tensor/layer executor, and OpenAI `/v1/chat/completions` + `/v1/embeddings` server).
2. **Protocol & API Endpoints**:
   - gRPC/QUIC service `umcf.ai.v1.InferenceCoordinator` and `umcf.ai.v1.ActivationRpcService` in `proto/umcf/ai/v1/inference.proto`.
   - Local loopback HTTP/SSE server (`127.0.0.1:11435/v1/*`) for OpenAI-compatible chat completions, embeddings, and model management.
3. **UI Screens**:
   - Flutter screens in `ui/flutter_app/lib/features/ai/`: `ModelRegistryScreen`, `InferencePlaygroundScreen`, `DistributedLayerTopologyWidget`, and `PrivateAiEnclaveSettingsSheet`.
   - Svelte 5 admin panels in `ui/admin_web/src/routes/ai/`: `+page.svelte` (Cluster Model Registry & Shards), `inference/+page.svelte` (Live Layer Partitioning & Token Throughput).

## 5. Acceptance Criteria
- Line and branch test coverage >= 95.0% across `crates/umcf-gguf`, `crates/umcf-llama-sys`, and `crates/umcf-ai` with a 100% pass rate.
- Deterministic GGUF verification and zero-memory-leak execution verified via Valgrind/ASan/LSan across 1,000+ consecutive inference sessions.
- Distributed multi-node layer split executes a real quantized GGUF model (e.g., TinyLlama-1.1B-Q4_K_M in CI and Llama-3-8B-Q4_K_M on hardware) across >= 2 nodes with identical greedy-decoded tokens (`temperature = 0.0`) compared to single-node execution.
- Private AI local-only mode provably blocks all outbound P2P tensor/prompt packets at both the scheduler and socket filter levels.

## 6. Required Tests
- **Unit (`[UNIT]`)**: GGUF v2/v3 header & tensor info parsing, KV-cache memory formulas, layer partitioning ILP/greedy solver, XChaCha20-Poly1305 prompt encryption, SSE chunk serialization.
- **Integration (`[INT]`)**: Resumable P2P model shard transfer over QUIC, end-to-end local `llama.cpp` inference via OpenAI `/v1/chat/completions`, RBAC model access enforcement.
- **Simulation (`[SIM]`)**: 4-node heterogeneous cluster layer partitioning and mid-generation worker disconnect failover using virtual network namespaces.
- **Hardware-Required (`[HW]`)**: Vulkan/Metal/CUDA/NEON backend acceleration benchmarks and thermal throttling fallback on real mobile/desktop GPUs.
- **Manually Verified (`[MANUAL]`)**: Flutter & Svelte 5 live streaming chat UI responsiveness, token-per-second telemetry charts, and Private AI badge verification.

## 7. Required Evidence & Artifacts
- `artifacts/stage-05/coverage/lcov.info` showing >= 95% coverage.
- `artifacts/stage-05/benchmarks/backend_throughput.json` (tokens/sec & prompt eval ms/token per backend).
- `artifacts/stage-05/distributed_parity_report.json` proving bit-exact greedy token parity between single-node and 3-node pipeline-parallel execution.
- `artifacts/stage-05/private_enclave_pcap_audit.json` proving zero non-loopback packets during Private AI mode.

## 8. Documentation, `TODO.md` & `CHANGELOG.md` Updates
- Update `docs/architecture/AI_INFERENCE_ENGINE.md` with GGUF store layout, backend selection state machine, and RPC activation wire format.
- Mark all `S5.P1.*`, `S5.P2.*`, and `S5.P3.*` items `[x]` in `TODO.md` only after evidence artifacts are generated and verified.
- Record Stage 05 completion in `CHANGELOG.md` under `[Unreleased] -> Added (Stage 05 - Distributed AI & Private Enclave)`.

## 9. Rollback Criteria & Completion Gate
- **Rollback Criteria**: Any memory corruption across the `llama.cpp` FFI boundary, segfault on GPU driver OOM, divergence in greedy token output across distributed nodes, or leak of plaintext prompts over non-encrypted/unauthorized peers triggers immediate rollback of the offending subphase.
- **Completion Gate**: Stage 06 (`stage-06`) may not begin until `S5_P1_PROMPT.md`, `S5_P2_PROMPT.md`, and `S5_P3_PROMPT.md` all pass their completion gates with zero open P0/P1 defects.
