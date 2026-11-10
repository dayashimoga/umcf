# Subphase S5.P3.SP3 Execution Prompt: Private AI Local-Only Mode, Prompt Encryption & OpenAI-Compatible Local API

## 1. Role & Context
You are the AI Privacy Enclave & Developer API Engineer for UMCF. In Subphase `S5.P3.SP3`, you will implement the **Private AI Privacy Enclave** (`crates/umcf-ai/src/private_enclave.rs`) and the **OpenAI-Compatible Local HTTP/SSE API Server** (`crates/umcf-ai/src/openai_api.rs`). Users must have cryptographic certainty that sensitive prompts in `StrictLocalOnly` mode never leave their device, and developers/tools must be able to point any OpenAI SDK client to `http://127.0.0.1:11435/v1` for chat completions, tool calling, and embeddings.

## 2. Prerequisites & Dependencies
- `S5.P2.SP3` (`InferenceEngine`, `KvCacheManager`) and `S5.P3.SP2` (`DistributedInferenceCoordinator`).
- `crates/umcf-crypto` (`XChaCha20Poly1305`, HKDF-SHA256, `zeroize::ZeroizeOnDrop`).

## 3. Implementation Tasks
1. **Implement Private AI Privacy Enclave (`crates/umcf-ai/src/private_enclave.rs`)**:
   - Define `PrivacyEnclaveMode`:
     - `StrictLocalOnly`: Hard-blocks `DistributedLayerPartitioner` and `DistributedInferenceCoordinator`; binds inference strictly to local CPU/GPU/NPU; trips a hard circuit breaker in `umcf-net` rejecting any outbound `umcf.ai.v1.*` RPC stream for the session.
     - `TrustedPersonalDevicesOnly`: Allows distributed layer partitioning *only* across devices signed by the same user's root identity key (excluding guest or marketplace nodes).
     - `ClusterEncrypted`: Allows cluster-wide execution with ephemeral per-session X25519 key exchange and XChaCha20-Poly1305 payload encryption on all activation packets.
   - Implement `SecretPromptBuffer` and `EphemeralInferenceSession`:
     - All prompt strings, token ID vectors, and KV-cache scratch buffers in Private Enclave sessions implement `ZeroizeOnDrop` so physical RAM pages are overwritten with zeros immediately when the request or session closes.
     - Disable all prompt/completion text logging in `tracing` spans (`[REDACTED_PRIVATE_ENCLAVE]`) and bypass disk history persistence unless explicitly saved to an encrypted local vault.
2. **Implement OpenAI-Compatible Local HTTP/SSE Server (`crates/umcf-ai/src/openai_api.rs`)**:
   - Build an `axum`-based loopback server (default `127.0.0.1:11435`, configurable CORS + optional Bearer API key authentication for local apps):
     - `GET /v1/models` and `GET /v1/models/{model}`: Returns loaded and available content-addressed GGUF models in OpenAI Model list format, enriched with `x_umcf_privacy_mode` and `x_umcf_cluster_nodes`.
     - `POST /v1/chat/completions`:
       - Supports `stream: true` (`text/event-stream` SSE with `chat.completion.chunk` deltas and final `data: [DONE]`) and `stream: false` (`chat.completion`).
       - Applies model-specific Jinja2 chat templates embedded in GGUF metadata (`tokenizer.chat_template`) via `minijinja` (supporting Llama-3, ChatML/Qwen, Mistral, Gemma, Phi-3, and tool/function calling schemas).
       - Supports `response_format: { type: "json_object" | "json_schema" }` wired to `S5.P2.SP3` GBNF grammar sampler.
       - Supports custom header `X-UMCF-Privacy-Mode: strict-local | trusted-devices | cluster-encrypted`.
     - `POST /v1/completions`: Legacy text completion endpoint (streaming and non-streaming).
     - `POST /v1/embeddings`: Computes L2-normalized float or base64 embeddings (`llama_get_embeddings_seq` / pooling modes `Mean`, `Cls`, `Last`) for RAG pipelines.
3. **UI Integration**:
   - Flutter: `PrivateAiEnclaveSettingsSheet` & shield toggle in `InferencePlaygroundScreen` (displays green lock shield with live proof counter: `Outbound AI Packets: 0 bytes`).
   - Svelte 5: `OpenAiApiStatusCard.svelte` showing local endpoint URL, one-click cURL / Python / TypeScript snippet generator, active API keys, and live request metrics.

## 4. Acceptance Criteria
- >= 95% line/branch coverage in `private_enclave.rs` and `openai_api.rs`; 100% test pass rate.
- In `StrictLocalOnly` mode, attempting to invoke a model that only fits across multiple nodes returns HTTP `422 Unprocessable Entity` (`"Model requires distributed cluster split, which is forbidden in StrictLocalOnly enclave mode"`) and emits 0 network bytes on any P2P interface.
- After an `EphemeralInferenceSession` is dropped, scanning the allocator buffer slice confirms the plaintext prompt bytes have been zeroed (`0x00`).
- Official OpenAI Python (`openai>=1.0`) and TypeScript (`openai>=4.0`) SDK client test suites pass against `http://127.0.0.1:11435/v1` for streaming chat, non-streaming chat, JSON schema structured outputs, tool calls, and embeddings.

## 5. Required Tests
- **`[UNIT]`**:
  - `test_minijinja_chat_template_rendering_llama3_chatml_gemma_tools`
  - `test_secret_prompt_buffer_zeroize_on_drop`
  - `test_strict_local_only_guard_blocks_remote_partition_plan`
  - `test_openai_sse_chunk_serialization_and_done_sentinel`
- **`[INT]`**:
  - `test_openai_chat_completions_streaming_and_non_streaming_e2e`
  - `test_openai_embeddings_l2_normalization_and_cosine_similarity`
  - `test_strict_local_only_zero_network_egress_verification`: Run 10 chat completions in `StrictLocalOnly` mode while capturing P2P transport counters, asserting `ai_rpc_bytes_sent == 0`.
  - `test_tracing_log_redaction_in_private_enclave_mode`: Assert secret canary string `"CANARY_SECRET_99412"` never appears in captured log output.
- **`[SIM]`**:
  - 50 concurrent SSE streaming clients connecting to `/v1/chat/completions` with abrupt TCP client disconnects verifying immediate `ggml_abort_callback` triggering.
- **`[HW]`**:
  - Verify end-to-end local RAG workflow (generate embeddings via `/v1/embeddings` + stream answer via `/v1/chat/completions`) on target device.
- **`[MANUAL]`**:
  - Enable `StrictLocalOnly` shield in Flutter Chat UI, send a prompt, verify green Enclave Lock indicator, and test a `curl http://127.0.0.1:11435/v1/chat/completions` command copied from the Svelte 5 dashboard.

## 6. Required Evidence & Artifacts
- `artifacts/stage-05/p3/sp3_openai_conformance_report.json`
- `artifacts/stage-05/p3/sp3_private_enclave_zeroize_and_pcap_audit.json`

## 7. Documentation, `TODO.md` & `CHANGELOG.md` Updates
- Document Private AI Enclave guarantees and OpenAI API compatibility reference in `docs/architecture/AI_INFERENCE_ENGINE.md` (Section 6) and `docs/api/OPENAI_COMPATIBILITY.md`.
- Mark `S5.P3.SP3`, Phase `S5.P3`, and **Stage 05** complete in `TODO.md` and `CHANGELOG.md`.

## 8. Rollback Criteria & Completion Gate
- **Rollback**: If any plaintext prompt token or string appears in logs during Private Enclave mode, if `StrictLocalOnly` ever contacts a remote peer, or if SSE streams fail to trigger kernel cancellation when the HTTP client disconnects.
- **Completion Gate**: All Stage 05 gates satisfied (`S5.P1`, `S5.P2`, `S5.P3` at >= 95% coverage, 100% pass rate). Stage 06 (`stage-06`) is now unlocked.
