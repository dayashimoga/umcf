# Phase S5.P3 Execution Prompt: Distributed Pipeline/Tensor Inference & Private AI Privacy Enclave

## 1. Role & Context
You are the Distributed AI Systems & Privacy Architect for UMCF. Phase `S5.P3` unlocks UMCF's flagship capability: pooling memory and compute across heterogeneous mobile, desktop, and server devices to run large LLMs that exceed any single device's RAM/VRAM, while also providing a mathematically enforced **Private AI Enclave** and an **OpenAI-compatible local API** (`127.0.0.1:11435/v1/*`) so any existing AI tool or app works out of the box.

This phase consists of three sequential subphases:
1. `S5_P3_SP1.md`: Memory-Aware Layer Partitioning Across Heterogeneous Nodes
2. `S5_P3_SP2.md`: Distributed `llama.cpp` RPC Activation Transfer & Fault Recovery
3. `S5_P3_SP3.md`: Private AI Local-Only Mode, Prompt Encryption & OpenAI-Compatible Local API

## 2. Prerequisites & Dependencies
- Completion of Phases `S5.P1` (Content-addressed GGUF store + selective layer download) and `S5.P2` (Native `llama.cpp` engine + backend selector + KV cache).
- Stage 2 & 4 (`umcf-net` QUIC mTLS streams, `umcf-scheduler` node capability & thermal scoring).

## 3. Implementation Tasks
1. **Subphase S5.P3.SP1 (`crates/umcf-ai/src/partitioner.rs`)**:
   - Implement `DistributedLayerPartitioner` solving the constrained layer assignment problem across $N$ heterogeneous cluster nodes based on usable RAM/VRAM, compute throughput (TFLOPS / tok/s score), inter-node link latency/bandwidth, battery/thermal constraints, and trust policy.
2. **Subphase S5.P3.SP2 (`crates/umcf-ai/src/distributed_rpc.rs`)**:
   - Implement the distributed pipeline/RPC inference coordinator and worker service (`ActivationRpcService`) over mTLS QUIC streams, transferring compressed/quantized intermediate hidden-state activations (`[batch_size, seq_len, n_embd]`) across consecutive layer partitions (`blk.0..k` -> `blk.k+1..m` -> `blk.m+1..L`), with standby replica/checkpoint failover if a mobile node drops mid-generation.
3. **Subphase S5.P3.SP3 (`crates/umcf-ai/src/private_enclave.rs`, `src/openai_api.rs`)**:
   - Implement the **Private AI Enclave** (`PrivacyMode::StrictLocalOnly`, `PrivacyMode::TrustedPersonalDevicesOnly`, `PrivacyMode::ClusterEncrypted`) with end-to-end XChaCha20-Poly1305 prompt/activation encryption, zero-retention ephemeral memory wiping (`zeroize`), and a drop-in OpenAI-compatible HTTP/SSE server (`/v1/models`, `/v1/chat/completions`, `/v1/completions`, `/v1/embeddings`).

## 4. Acceptance Criteria
- >= 95% line/branch coverage across `partitioner.rs`, `distributed_rpc.rs`, `private_enclave.rs`, and `openai_api.rs`; 100% test pass rate.
- Bit-exact greedy decoding parity (`temperature = 0.0`, top-1 token match rate = 100%) between single-node inference and 2-, 3-, and 4-node distributed pipeline inference on the same GGUF model.
- When a worker node disconnects mid-inference during a distributed session, the coordinator recovers and continues token generation from the current KV-cache token position within <= 1.5 seconds by reassigning the lost layer range to a standby/head node.
- In `PrivacyMode::StrictLocalOnly`, network socket guards and scheduler assertions guarantee 0 bytes of prompt, KV-cache, or activation data ever leave the local host.

## 5. Required Tests
- **`[UNIT]`**: Layer partition solver optimality and memory safety bounds, activation tensor serialization/CRC, OpenAI request/SSE response schema compliance, `zeroize` memory scrubbing.
- **`[INT]`**: 3-node local cluster distributed inference parity test; mid-stream node kill and seamless failover; OpenAI Python/TypeScript client SDK integration test against `/v1/chat/completions` and `/v1/embeddings`.
- **`[SIM]`**: Heterogeneous 5-node cluster (2 phones + 1 laptop + 1 desktop + 1 tablet) under fluctuating Wi-Fi latency and simulated battery drain.
- **`[HW]`**: Real multi-device distributed LLM inference across a desktop and mobile device over LAN/Wi-Fi.
- **`[MANUAL]`**: Connect external OpenAI client / UMCF Flutter Chat UI to `http://127.0.0.1:11435/v1`, visualize the live multi-node layer topology ring in Flutter & Svelte 5, and toggle `StrictLocalOnly` mode.

## 6. Required Evidence & Artifacts
- `artifacts/stage-05/p3/distributed_greedy_parity_report.json`
- `artifacts/stage-05/p3/node_failover_recovery_timeline.json`
- `artifacts/stage-05/p3/openai_api_conformance_suite.json`
- `artifacts/stage-05/p3/private_enclave_network_isolation_audit.json`

## 7. Documentation, `TODO.md` & `CHANGELOG.md` Updates
- Update `docs/architecture/AI_INFERENCE_ENGINE.md` (Sections 4–6: Layer Partitioning, Distributed Activation RPC, Private AI & OpenAI API).
- Mark `S5.P3.SP1`–`S5.P3.SP3` and all of Stage 05 `[x]` in `TODO.md`; update `CHANGELOG.md`.

## 8. Rollback Criteria & Completion Gate
- **Rollback**: Any token divergence in greedy mode between single-node and distributed execution, any deadlock on worker disconnect, or any network packet leak in `StrictLocalOnly` mode.
- **Gate**: Stage 05 complete. Unlocks Stage 06 (`stage-06`).
