# Subphase S5.P3.SP2 Execution Prompt: Distributed `llama.cpp` RPC Activation Transfer & Fault Recovery

## 1. Role & Context
You are the Distributed Systems & RPC Data-Plane Engineer for UMCF. In Subphase `S5.P3.SP2`, you will implement the distributed pipeline-parallel and GGML-RPC execution engine in `crates/umcf-ai/src/distributed_rpc.rs`. This subsystem coordinates multi-node inference across the `DistributedPartitionPlan` produced in `S5.P3.SP1`, streaming intermediate hidden-state activation tensors over mTLS QUIC streams with optional FP16/Q8_0 activation compression, and performing automatic checkpoint/replica failover if any worker node drops mid-generation.

## 2. Prerequisites & Dependencies
- `S5.P3.SP1` (`DistributedPartitionPlan`, `LayerPartitionStage`).
- `S5.P2.SP1`–`SP3` (`SafeLlamaModel`, `SafeLlamaContext`, `KvCacheManager`).
- `crates/umcf-net` (mTLS QUIC persistent bidirectional streams, 0-RTT reconnection).

## 3. Implementation Tasks
1. **Implement Distributed Pipeline & GGML-RPC Protocol (`crates/umcf-ai/src/distributed_rpc.rs`)**:
   - Define the wire protocol messages in `proto/umcf/ai/v1/inference.proto`:
     - `InitPipelineStageRequest { session_id, model_id, stage_config, kv_params }`
     - `ForwardActivationPacket { session_id, seq_id, step_idx, token_positions: Vec<i32>, dtype: ActivationDtype, shape: [u32; 3], payload: Bytes, blake3_checksum: [u8; 32] }`
     - `ForwardActivationResponse { session_id, step_idx, output_activation_or_logits: Bytes, stage_compute_us: u64 }`
     - `CommitKvStep` / `RollbackKvStep` / `EvictSession`
   - Support two execution modes selectable by link bandwidth:
     - **Mode A (Pipeline Layer Stage Worker)**: Each worker loads only its assigned layer shard `[start_layer..=end_layer]` + local KV cache for those layers, evaluates its subgraph on incoming `ForwardActivationPacket`, and forwards the resulting hidden state to the next stage in the ring (single activation hop per stage boundary).
     - **Mode B (Native `ggml-rpc` Tensor Server Bridge)**: Tunnels `ggml-rpc` commands over UMCF's authenticated mTLS QUIC transport (replacing upstream unauthenticated TCP `ggml-rpc` sockets with mTLS + capability-token verification).
2. **Implement Optional Activation Quantization (`ActivationCodec`)**:
   - For WAN or constrained Wi-Fi links (< 100 Mbps), support lossless LZ4 or dynamic per-token FP16 / Q8_0 activation compression of the hidden state tensor `[n_tokens, n_embd]` before QUIC transmission, reducing wire payload by 2x–4x while preserving FP32/FP16 residual accuracy on LAN links.
3. **Mid-Generation Fault Recovery & Worker Failover (`PipelineFaultTolerance`)**:
   - Maintain a lightweight shadow layer mapping or coordinator fallback:
     - Every `ForwardActivationPacket` is tagged with `step_idx` and `token_positions`.
     - If Stage $i$ fails to respond within `stage_timeout_ms` or its QUIC stream resets:
       1. Coordinator aborts the in-flight step `step_idx` and sends `RollbackKvStep(step_idx)` to surviving stages.
       2. Coordinator promotes a standby peer (or loads the missing layer range on the head/surviving node using cached shards from `S5.P1.SP1`), replays the prompt/cached tokens for Stage $i$ (or restores Stage $i$'s periodic KV delta snapshot), and resumes token generation from `step_idx` transparently to the user stream.
4. **UI Integration**:
   - Flutter & Svelte 5: Live pulse animation along inter-node links in the topology view showing activation packets in flight, per-stage compute latency (ms) vs network transfer latency (ms), and failover event toast notification.

## 4. Acceptance Criteria
- >= 95% line/branch coverage in `crates/umcf-ai/src/distributed_rpc.rs`; 100% test pass rate.
- **Bit-Exact Greedy Parity**: With FP32/FP16 uncompressed activations and `temperature = 0.0`, a 3-node pipeline produces 100% identical token IDs over a 128-token generation compared to single-node local execution.
- **Security**: Zero unauthenticated TCP sockets—all distributed activation and RPC traffic runs strictly over UMCF mTLS QUIC streams with per-session capability tokens and BLAKE3 packet integrity checks.
- **Failover SLA**: Killing a middle worker node at token #32 of a 64-token stream triggers automatic failover and completes tokens #32..#64 within <= 1.5 seconds recovery overhead without dropping the client's SSE/stream connection.

## 5. Required Tests
- **`[UNIT]`**:
  - `test_activation_packet_serialization_and_blake3_checksum_validation`
  - `test_fp16_and_q8_activation_codec_roundtrip_snr`
  - `test_kv_step_commit_and_rollback_state_consistency`
- **`[INT]`**:
  - `test_3_node_pipeline_greedy_token_parity_with_single_node`: Spin up 3 in-process / localhost QUIC nodes holding layers `[0..7]`, `[8..15]`, and `[16..21]`, run greedy generation, and assert exact token ID vector equality against 1-node execution.
  - `test_worker_disconnect_mid_generation_seamless_failover`: Kill node #2 at token 15, verify coordinator rolls back uncommitted step 15, fails over layers `[8..15]` to standby node #4, and completes generation without stream error.
  - `test_unauthenticated_rpc_connection_rejected_by_mtls_guard`
- **`[SIM]`**:
  - Inject 20 ms RTT jitter and 2% packet loss on inter-stage links, verifying QUIC stream recovery and timeout adaptation.
- **`[HW]`**:
  - Execute distributed 2-node inference across two physical machines/devices and measure end-to-end tokens/sec and network bandwidth utilization.
- **`[MANUAL]`**:
  - Watch live stage-by-stage activation latency bars in Svelte 5 / Flutter UI while pulling the network cable / toggling Wi-Fi on a secondary worker node to observe live failover.

## 6. Required Evidence & Artifacts
- `artifacts/stage-05/p3/sp2_3node_greedy_parity_proof.json`
- `artifacts/stage-05/p3/sp2_mid_stream_failover_trace.json`

## 7. Documentation, `TODO.md` & `CHANGELOG.md` Updates
- Document the distributed activation wire protocol, mTLS security wrapper, and KV rollback/failover protocol in `docs/architecture/AI_INFERENCE_ENGINE.md` (Section 5).
- Mark `S5.P3.SP2` complete in `TODO.md` and update `CHANGELOG.md`.

## 8. Rollback Criteria & Completion Gate
- **Rollback**: If upstream `ggml-rpc` raw TCP socket is ever exposed without mTLS encapsulation, if greedy token output diverges from single-node reference, or if worker disconnect causes the coordinator to hang indefinitely.
- **Completion Gate**: All tests green with >= 95% coverage before proceeding to `S5.P3.SP3`.
