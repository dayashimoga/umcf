# Subphase S5.P2.SP3 Execution Prompt: Streaming Token Generation, KV-Cache Management & Cancellation

## 1. Role & Context
You are the LLM Runtime & Streaming Engine Engineer for UMCF. In Subphase `S5.P2.SP3`, you will implement the asynchronous streaming inference engine, multi-slot KV-cache prefix manager, grammar-constrained sampler, and deterministic sub-50ms cancellation system in `crates/umcf-ai/src/engine.rs` and `src/kv_cache.rs`.

## 2. Prerequisites & Dependencies
- `S5.P2.SP1` (`SafeLlamaModel`, `SafeLlamaContext`, `SafeLlamaBatch`, `SafeSamplerChain`).
- `S5.P2.SP2` (`BackendExecutionPlan`, `FallbackExecutor`).

## 3. Implementation Tasks
1. **Streaming Token Generation Engine (`crates/umcf-ai/src/engine.rs`)**:
   - Implement `InferenceEngine` running a dedicated high-priority compute worker loop communicating with async callers via bounded `tokio::sync::mpsc` channels (`StreamEvent::PromptProgress`, `StreamEvent::TokenDelta`, `StreamEvent::UsageMetrics`, `StreamEvent::Completed`, `StreamEvent::Cancelled`).
   - Implement chunked prompt evaluation (`n_ubatch = 256` or `512`) so long prompts (e.g., 8k–32k tokens) report granular prefill progress (`prompt_tokens_processed / total_prompt_tokens`) and remain responsive to cancellation between micro-batches.
   - Implement stateful UTF-8 incremental detokenizer (`Utf8TokenAssembler`) that buffers incomplete multi-byte UTF-8 sequences (e.g., emoji or CJK characters split across two BPE tokens) and emits valid `String` deltas without `U+FFFD` replacement corruption.
   - Support full sampling configuration: `temperature`, `top_k`, `top_p`, `min_p`, `repeat_penalty`, `frequency_penalty`, `presence_penalty`, `seed` (deterministic PRNG), stop token strings/IDs, and GBNF / JSON Schema grammar constraints (`llama_sampler_init_grammar`).
2. **Multi-Slot KV-Cache & Prefix Reuse Manager (`crates/umcf-ai/src/kv_cache.rs`)**:
   - Implement `KvCacheManager` managing `k` parallel sequence slots (`llama_seq_id`) within a context:
     - **Longest Common Prefix (LCP) Matching**: Before prefilling a new conversation turn, match its token prefix against cached sequence slots (`llama_kv_cache_seq_cp` / `llama_kv_cache_seq_rm`) to skip re-evaluating shared system prompts and prior multi-turn chat history.
     - **Context Shifting (`RoPE` / Sink Token Shifting)**: When a conversation exceeds `n_ctx` and `context_shift_enabled = true`, retain the first `n_keep` system tokens and shift the newest `(n_ctx - n_keep) / 2` tokens via `llama_kv_cache_seq_add`.
     - **Encrypted Disk Session Persistence**: Save and restore compressed KV-cache slot state (`llama_state_seq_get_data` / `llama_state_seq_set_data`) encrypted with XChaCha20-Poly1305.
3. **Sub-50ms Cooperative C++ Kernel Cancellation**:
   - Wire `ggml_abort_callback` on `SafeLlamaContext` to an `Arc<AtomicBool>` (`cancel_flag`) associated with the active request's `CancellationToken`.
   - When a client disconnects, clicks "Stop", or a scheduler preemption occurs, set `cancel_flag.store(true, Ordering::Release)` so the active C++ graph evaluation aborts immediately inside `ggml` and rolls back uncommitted KV-cache tokens for that sequence slot.
4. **UI Integration**:
   - Flutter `InferencePlaygroundScreen` & Svelte 5 `InferenceConsole.svelte`: Live streaming markdown renderer, prefill progress bar, KV-cache hit-rate badge (e.g. `KV Prefix Reuse: 92% (1,420 tokens saved)`), JSON Schema mode toggle, and instant Stop button.

## 4. Acceptance Criteria
- >= 95% line/branch coverage in `engine.rs` and `kv_cache.rs`; 100% test pass rate.
- Multi-turn chat follow-up sharing a 1,000-token prefix reuses >= 99% of cached KV tokens, reducing time-to-first-token (TTFT) by >= 80% compared to cold prefill.
- Triggering `CancellationToken::cancel()` during either prompt prefill or token decode stops `llama_decode` and frees the worker slot in <= 50 ms without corrupting other sequences in the KV cache.
- JSON Schema grammar mode guarantees 100% syntactically valid JSON output conforming to the user's schema.

## 5. Required Tests
- **`[UNIT]`**:
  - `test_utf8_token_assembler_multibyte_emoji_and_cjk_split`
  - `test_kv_cache_longest_common_prefix_matching_and_lru_eviction`
  - `test_json_schema_to_gbnf_converter_and_sampler`
  - `test_context_shift_preserves_sink_tokens`
- **`[INT]`**:
  - `test_multi_turn_chat_kv_prefix_cache_speedup`
  - `test_mid_decode_cancellation_via_ggml_abort_callback_under_50ms`
  - `test_kv_session_encrypted_disk_save_and_restore_bit_exact_continuation`
- **`[SIM]`**:
  - 32 concurrent client requests multiplexing across 4 KV-cache sequence slots with random disconnects and cancellations.
- **`[HW]`**:
  - Benchmark TTFT (ms) and decode throughput (tokens/s) on target hardware with and without KV prefix caching.
- **`[MANUAL]`**:
  - Stream a multi-turn conversation with emojis, code blocks, and JSON-schema mode in Flutter & Svelte 5, testing the Stop button mid-sentence.

## 6. Required Evidence & Artifacts
- `artifacts/stage-05/p2/sp3_kv_prefix_reuse_benchmark.json`
- `artifacts/stage-05/p2/sp3_cancellation_latency_histogram.json`

## 7. Documentation, `TODO.md` & `CHANGELOG.md` Updates
- Document KV-cache prefix tree, context shifting, and `ggml_abort_callback` semantics in `docs/architecture/AI_INFERENCE_ENGINE.md` (Section 3).
- Mark `S5.P2.SP3` and Phase `S5.P2` complete in `TODO.md`; update `CHANGELOG.md`.

## 8. Rollback Criteria & Completion Gate
- **Rollback**: If cancelling an in-flight request leaves partial unaligned KV state that produces garbled tokens on the subsequent request, or if multi-byte UTF-8 tokens ever emit invalid UTF-8 strings.
- **Completion Gate**: Phase `S5.P2` completion gate passed (`S5_P2_SP1`..`SP3` green, >= 95% coverage). Proceed to Phase `S5.P3`.
