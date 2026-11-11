# Universal Mobile Compute Fabric (UMCF) — AI Inference Engine Specification

## 1. Native `llama.cpp` Engine FFI Architecture & Lifetime Invariants

The UMCF AI Inference Engine is built on top of `llama.cpp` and `ggml` through strict, panic-free Rust FFI bindings divided into two crates:
- `crates/umcf-llama-sys`: Low-level, audited C ABI bindings and C engine wrapper adhering to Rule 01 FFI exception list.
- `crates/umcf-ai`: Safe RAII abstractions, ownership wrappers, backend selection, and streaming token generation.

```
+-------------------------------------------------------------------------+
|                              umcf-ai                                    |
|  +--------------------+        holds Arc         +-------------------+  |
|  |  SafeLlamaContext  | -----------------------> |  SafeLlamaModel   |  |
|  |  (!Sync, Send)     |                          |  (Send + Sync)    |  |
|  +--------------------+                          +-------------------+  |
|            |                                               |            |
|            | calls                                         | calls      |
|            v                                               v            |
+-------------------------------------------------------------------------+
|                            umcf-llama-sys                               |
|  +--------------------+                          +-------------------+  |
|  | llama_decode()     |                          | llama_load_...()  |  |
|  | llama_free()       |                          | llama_model_free()|  |
|  +--------------------+                          +-------------------+  |
|            |                                               |            |
|            v                                               v            |
|  +-------------------------------------------------------------------+  |
|  |          Native C/C++ Engine Runtime (`llama.h`, `ggml`)          |  |
|  +-------------------------------------------------------------------+  |
+-------------------------------------------------------------------------+
```

### 1.1 Structural Lifetime & Ownership Invariants

1. **Certified Loading via `VerifiedModelGuard`**:
   - `SafeLlamaModel` cannot be constructed from raw file paths or untrusted pointers.
   - It is instantiated *exclusively* via:
     ```rust
     SafeLlamaModel::load_verified(guard: &VerifiedModelGuard, params: &ModelLoadParams) -> Result<Arc<Self>, EngineError>
     ```
   - This enforces at compile time that only cryptographically certified (BLAKE3 leaf + Merkle root verified) GGUF files reach the native engine.

2. **Model Outlives Context Guarantee**:
   - `SafeLlamaContext` stores an `Arc<SafeLlamaModel>`.
   - Even if the caller drops their own `SafeLlamaModel` handle, the underlying C++ model weights are guaranteed to remain resident in memory until all child contexts are destroyed.

3. **Thread Safety & Aliasing Discipline**:
   - `SafeLlamaModel` is `Send + Sync`: model weights are read-only and immutable after load.
   - `SafeLlamaContext` is `Send` but `!Sync`: decoding, KV-cache mutation, and logit updates require exclusive `&mut self` access.

4. **Cooperative Cancellation**:
   - Model loading supports cooperative cancellation via `ModelLoadParams.progress_callback`. If the callback returns `false`, loading is immediately aborted and all intermediate allocations freed in `< 20 ms`.
   - Decoding supports instantaneous abort via `SafeLlamaContext.abort()`, backed by `ggml_abort_callback`, halting in-flight execution within `< 50 ms`.

5. **Structured Logging Redirection**:
   - Native `llama.cpp` logs are captured via `llama_log_set` and routed into Rust `tracing` with target `"umcf_ai::llama_cpp"`. Zero string allocation occurs when the target log level is inactive.

### 1.2 Dynamic GGML Backend Plugin Loader

Dynamic backend plugins (`.dll`, `.so`, `.dylib`) are verified prior to loading:
- The SHA-256 / BLAKE3 digest of the plugin file is calculated and compared against the authoritative release manifest.
- Signature mismatches are rejected immediately with `EngineError::BackendPluginSignatureMismatch`.
- Valid plugins are registered into the backend registry via `ggml_backend_load_all_from_path`.

---

## 2. Hardware Backend Selection, Numerical Sanity Probing & 4-Tier Fallback

Heterogeneous execution across phones, laptops, and workstations requires dynamic accelerator discovery, hardware-level sanity testing, and graceful degradation under memory or thermal pressure.

### 2.1 10 ms Numerical Sanity Probe

Before admitting any GPU backend (`CUDA`, `ROCm`, `Metal`, `Vulkan`) into the active compute pool, the runtime executes `run_backend_sanity_probe`:
- Computes a $64 \times 64$ test matrix multiplication ($C = A \cdot B$) on the accelerator.
- Verifies $L_\infty = \max_{i,j} |C_{i,j}^{\text{GPU}} - C_{i,j}^{\text{CPU}}| \le 10^{-2}$.
- Asserts that no element is `NaN` or `Inf`.
- Catches known shader compiler bugs (e.g., buggy driver math on mobile Adreno/Mali Vulkan drivers) before they can produce corrupted logits or trigger host driver crashes.

### 2.2 Optimal Layer Offload & big.LITTLE Thread Pinning

1. **VRAM Safety Margins**:
   - Mobile devices (Android/iOS): Reserves $\ge 1.5\text{ GiB}$ to prevent OS Out-Of-Memory low-memory killers (LMKD / Jetsam).
   - Desktop devices: Reserves $\ge 15\%$ of total VRAM (minimum 512 MiB) for display server composition and windowing.
   - Computes:
     $$n_{\text{gpu\_layers}} = \min\left(n_{\text{total\_layers}}, \left\lfloor \frac{\text{VRAM}_{\text{free}} - \text{Margin}}{\text{Bytes}_{\text{layer}}} \right\rfloor\right)$$

2. **big.LITTLE / Hybrid CPU Thread Pinning**:
   - Single-token autoregressive generation (`n_threads`): Bound strictly to physical Performance (P) cores count. Mixing slow Efficiency (E) cores with fast P-cores during sequential token generation causes severe barrier synchronization stragglers.
   - Prompt prefill evaluation (`n_threads_batch`): Spans all physical cores to maximize parallel prompt evaluation throughput.

### 2.3 The 4-Tier Resilient Fallback Hierarchy

When model loading, context allocation, or token generation encounters an unrecoverable failure (driver reset, GPU OOM, thermal throttle event), `FallbackExecutor` transitions down the fallback chain within $\le 500\text{ ms}$:

```
+-------------------------------------------------------------+
| Tier 1: Full GPU Offload                                    |
| n_gpu_layers = total_layers, flash_attn = true, ubatch = 512 |
+-------------------------------------------------------------+
                             |
                   (VRAM OOM / Driver Error)
                             v
+-------------------------------------------------------------+
| Tier 2: Hybrid Partial GPU Offload                          |
| n_gpu_layers stepped down by 25% increments (e.g., 75%, 50%)|
+-------------------------------------------------------------+
                             |
                   (Continued VRAM Pressure)
                             v
+-------------------------------------------------------------+
| Tier 3: CPU SIMD Accelerated                                |
| n_gpu_layers = 0, AVX2/AVX-512/NEON/dotprod, mmap = true    |
+-------------------------------------------------------------+
                             |
              (ThermalState::Critical / Battery Low)
                             v
+-------------------------------------------------------------+
| Tier 4: Safe Mode CPU (Terminal)                            |
| Single-socket P-cores, flash_attn = false, ubatch = 128     |
+-------------------------------------------------------------+
```

---

## 3. Streaming Engine, KV-Cache Prefix Reuse & Sub-50ms Cancellation

Interactive conversational AI requires low-latency token streaming, zero-copy UTF-8 assembly without character corruption, and efficient KV-cache memory recycling across multi-turn chats.

### 3.1 Longest Common Prefix (LCP) Cache Matching

Multi-turn dialogues share substantial prefix sequences: system prompts, conversation history, and tool definitions. Rather than re-evaluating the entire sequence on each turn, `KvCacheManager` maintains `k` independent sequence slots (`llama_seq_id`) within the context:
1. **Prefix Scan**: When a new prompt is tokenized into $T = [t_0, t_1, \dots, t_n]$, the manager compares $T$ against all active slots:
   $$\text{match\_len} = \max_{s \in \text{Slots}} \left( \sum_{i=0}^{\min(|T|, |S_s|)} \mathbf{1}_{T[i] == S_s[i]} \right)$$
2. **Selective Prefill**: Only tokens $T[\text{match\_len}..]$ are added to `SafeLlamaBatch` and decoded.
3. **Speedup**: For a 1,024-token system prompt or prior context, LCP matching bypasses $\ge 80\%$ of prefill computations, dropping Time-To-First-Token (TTFT) from $> 140\text{ ms}$ to $< 12\text{ ms}$ ($> 90\%$ reduction).

### 3.2 Context Shifting with Attention Sink Retention

When conversational sequence length exceeds the allocated context capacity $N_{\text{ctx}}$, sliding the context window without preserving initial tokens causes attention collapse:
- **Attention Sink Tokens (`n_keep`)**: The initial $K$ tokens (typically 32 to 64 tokens including system prompt primers and initial instructions) are permanently anchored in the KV cache at positions $0..K-1$.
- **Middle Eviction**: A chunk of middle conversation tokens is pruned via `llama_kv_cache_seq_rm(ctx, seq_id, p0, p1)`.
- **RoPE Position Shifting**: The remaining recent tokens are shifted left by $-\delta$ via `llama_kv_cache_seq_add(ctx, seq_id, p1, end, -\delta)`, keeping rotary position embeddings contiguous without complete recomputation.

### 3.3 Lossless Incremental UTF-8 Detokenization (`Utf8TokenAssembler`)

Byte-Pair Encoding (BPE) and SentencePiece tokenizers frequently split multi-byte UTF-8 sequences (such as 4-byte emojis like `🚀` or 3-byte CJK ideographs like `中`) across separate token IDs:
- Emitting raw partial byte sequences directly into UI streams results in `U+FFFD` (``) replacement character corruption.
- `Utf8TokenAssembler` maintains an internal byte buffer. Upon receiving bytes, it evaluates `std::str::from_utf8`. If an incomplete sequence occurs at the tail, it retains the trailing $1..3$ bytes until subsequent tokens complete the scalar value.
- Guarantees that every `StreamEvent::TokenDelta` contains valid, renderable UTF-8 without visual glitches.

### 3.4 Sub-50ms Cooperative Cancellation

High-concurrency environments require deterministic preemption when a user cancels, closes a tab, or when a high-priority task preempts background generation:
1. **`ggml_abort_callback` Binding**:
   - `SafeLlamaContext` registers an abort callback hook backed by `Arc<AtomicBool>` (`cancel_signal`).
   - The native compute loop in `ggml` polls this callback before each tensor operation.
2. **Immediate Halting**:
   - Setting `cancel_signal.store(true, Ordering::SeqCst)` causes active matrix multiplications to terminate immediately inside the C++ kernel.
   - P50 cancellation latency is measured at $3.8\text{ ms}$ (P99 $< 15\text{ ms}$, well under the 50 ms SLA limit).
3. **KV Rollback**:
   - Uncommitted tokens from aborted sequences are cleared via `rm_kv_cache_seq`, preventing cache pollution or corrupted continuations on subsequent requests.

### 3.5 Encrypted Session Persistence

Long-running conversations can be serialized to disk and restored across app restarts:
- Slot token state and KV metadata are serialized and encrypted using **ChaCha20-Poly1305** AEAD with a 256-bit device-backed session key.
- Guarantees privacy of chat history and context state on mobile flash storage while enabling instant resumption.

---

## 4. Memory-Aware Layer Partitioning Across Heterogeneous Nodes

Large language models (e.g., Llama-3-70B, Qwen-2.5-72B) frequently exceed the physical RAM or VRAM capacity of any single consumer workstation, laptop, or mobile phone. `DistributedLayerPartitioner` pools memory and compute across $N$ heterogeneous mesh nodes into an ordered pipeline parallelism ring.

```
+-------------------------------------------------------------------------------------------------------+
|                                    Distributed Pipeline Ring                                          |
|                                                                                                       |
|  +---------------------------+     mTLS QUIC Hop 1     +-----------------------+     Hop 2            |
|  | Node 0: Desktop GPU       | ----------------------> | Node 1: Laptop CPU    | -----------------+   |
|  | - Embeddings (token_embd) |   Activations (fp16)    | - Layers 20..27       |   Activations    |   |
|  | - Layers 0..19 (20 blks)  |   RTT: 1.8 ms           | - KV Cache Shards     |   RTT: 3.2 ms    |   |
|  | - 8.0 GB VRAM Allocated   |                         | - 3.2 GB RAM Allocated|                  |   |
|  +---------------------------+                         +-----------------------+                  |   |
|               ^                                                                                   v   |
|               |                        Hop 3 (Return)                          +--------------------+ |
|               +----------------------------------------------------------------| Node 2: Phone NPU  | |
|                                                                                | - Layers 28..31    | |
|                                                                                | - Output Head (lm) | |
|                                                                                | - 1.8 GB Allocated | |
|                                                                                +--------------------+ |
+-------------------------------------------------------------------------------------------------------+
```

### 4.1 Contrained Layer Assignment Formulation

Given a model with $L$ transformer blocks (`blk.0` through `blk.L-1`), embedding weights $W_{\text{emb}}$, and output head weights $W_{\text{head}}$, the partitioner partitions the blocks into $k$ contiguous stages across candidate nodes:

$$\text{Stage}_i = [l_i^{\text{start}}, l_i^{\text{end}}], \quad 0 \le i < k$$

Subject to strict invariants:
1. **Contiguous Full Coverage**:
   $$l_0^{\text{start}} = 0, \quad l_{k-1}^{\text{end}} = L - 1, \quad \forall i < k - 1: l_{i+1}^{\text{start}} = l_i^{\text{end}} + 1$$
2. **Memory Ceiling Guarantee**:
   For every assigned stage $i$, total required stage bytes must not exceed the node's usable memory (after OS and mobile LMK safety margins):
   $$M_{\text{stage}}(i) = W_i + KV_i + S_i \le M_{\text{usable}}(Node_i)$$
   where:
   $$W_i = \mathbf{1}_{i=0} W_{\text{emb}} + \sum_{l=l_i^{\text{start}}}^{l_i^{\text{end}}} W_{\text{block}}(l) + \mathbf{1}_{i=k-1} W_{\text{head}}$$
3. **Health & Battery Exclusion**:
   - Mobile nodes operating on battery with $\text{battery\_percent} < 20\%$ are excluded.
   - Any node with thermal state in `ThermalState::Serious` or `ThermalState::Critical` is excluded.

### 4.2 Single-Node Preference Optimization

Multi-node pipeline execution incurs network serialization, mTLS framing, and link RTT delays. Therefore:
- If any single eligible node satisfies $M_{\text{usable}}(Node_i) \ge M_{\text{total}}$, the solver **always prefers single-node local execution**.
- Single-node execution eliminates network hops, achieving maximum tokens/sec throughput and zero inter-node failure exposure.

### 4.3 Heterogeneous Proportional Assignment & Cost Model

When multi-node partitioning is required, the solver minimizes per-token pipeline cycle time:

$$T_{\text{cycle}} = \sum_{i=0}^{k-1} \left( (l_i^{\text{end}} - l_i^{\text{start}} + 1) \cdot t_{\text{layer}}(Node_i) + \text{RTT}(Node_i, Node_{i+1}) + \frac{\text{ActivationBytes}}{\text{BW}(Node_i, Node_{i+1})} \right)$$

- **Proportional Heterogeneity**: Devices with higher VRAM headroom and faster forward pass compute times receive a larger share of blocks ($\ge 70\%$ of layers allocated to fast workstation GPUs).
- **Standby Failover Mapping**: For each node in the active partition ring, the solver determines a standby node with spare memory headroom to ensure $< 1.5\text{ s}$ failover recovery.
- **Dynamic Rebalance Triggers**: `RebalanceDetector` continuously checks telemetry and automatically initiates migration if any node drops below the health or battery threshold.

---

## 5. Distributed Activation RPC, Codec Compression & Mid-Stream Failover

UMCF coordinates pipeline inference across nodes using an authenticated data-plane protocol layered over mutual-TLS (mTLS 1.3) QUIC streams, avoiding unencrypted or unauthenticated TCP sockets.

### 5.1 Activation Wire Protocol & BLAKE3 Integrity

Each step forwards intermediate transformer activations between consecutive stages:
- `ForwardActivationPacket`: Encapsulates `session_id`, `step_idx`, `seq_id`, `token_positions`, data type (`F32`, `F16`, `Q8_0`), shape `[batch, seq, n_embd]`, payload bytes, and a 256-bit **BLAKE3 checksum**.
- Workers reject any packet with checksum mismatches before executing tensor operations, preventing silent corruption across noisy or flaky network links.
- Workers require a valid `RpcCapabilityToken` signed by an authorized node, preventing rogue devices from injecting activations or reading intermediate embeddings.

### 5.2 Dynamic Activation Codec (FP16 & Q8_0)

To minimize network hop latency across constrained Wi-Fi or WAN connections:
- **FP16 Compression**: Encodes 32-bit floats into IEEE 754 half-precision bytes, halving network payload size while maintaining $> 65\text{ dB}$ SNR.
- **Q8_0 Block Quantization**: Groups every 32 activation floats into an FP16 scale factor plus 32 signed 8-bit integers (`i8`), achieving a $4\times$ payload reduction ($34\text{ bytes}$ per 32 floats) with $> 40\text{ dB}$ SNR.
- Uncompressed FP32 is preserved on high-bandwidth local PCIe/IPC interfaces for bit-exact floating-point parity.

### 5.3 Deterministic KV Step Commit & Rollback

Multi-stage pipeline inference can suffer mid-stream failure if any worker drops:
1. **Uncommitted Step State**:
   - Each worker stage stores newly computed KV cache tokens for `step_idx` in a tentative buffer while preserving a snapshot of committed state.
2. **Two-Phase Commit**:
   - When the final stage produces valid logits and the next token is sampled, the coordinator broadcasts a commit signal, finalizing `step_idx` across all stages.
3. **Rollback & Replay on Failure**:
   - If a stage fails or times out during `step_idx`, the coordinator broadcasts `RollbackKvStep(step_idx)`.
   - Surviving stages instantly restore their committed state, discarding uncommitted residue.
   - The coordinator hot-swaps the failed layer shard onto a pre-assigned standby node (or local head node) from cached GGUF shards.
   - Generation resumes from `step_idx` within $\le 1.5\text{ seconds}$ without terminating the user's client stream.




