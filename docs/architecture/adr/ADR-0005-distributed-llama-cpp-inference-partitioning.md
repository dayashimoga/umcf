# ADR-0005: Distributed `llama.cpp` Inference via Layer-Partitioned Pipeline Parallelism and Activation Quantization

- **Status**: Accepted
- **Date**: 2026-11-10
- **Deciders**: UMCF Core Architecture Team
- **Related Components**: `umcf-engine-ai`, `umcf-scheduler`, `umcf-engine-storage`, `umcf-worker`

---

## 1. Context and Problem Statement

A primary use case of UMCF is running Large Language Models (LLMs) such as Llama-3 (8B to 70B), Qwen-2.5, Mistral, and DeepSeek across consumer desktops, laptops, and smartphones. 

In distributed LLM inference, two primary parallelism strategies exist:
1. **Tensor Parallelism (TP)**: Splitting individual weight matrices (e.g., attention $W_Q, W_K, W_V, W_O$ and MLP projections) across nodes, requiring an `AllReduce` synchronization barrier **twice per transformer layer** ($2 \times L$ network round-trips per token).
2. **Layer-Wise Pipeline Parallelism (PP)**: Assigning contiguous ranges of complete transformer layers ($L_{\text{start}} \dots L_{\text{end}}$) to each node in a ring/chain, requiring only **one point-to-point activation transfer per node boundary** ($k$ network hops per token, where $k \ll 2L$).

## 2. Quantitative Comparison on Consumer Networks

Consider a 32-layer model ($L = 32$) distributed across $k = 3$ nodes connected over Wi-Fi 6 with $\text{RTT} = 2.0\text{ ms}$:
- **Tensor Parallelism**: Requires $2 \times 32 = 64$ all-reduce barriers per token $\implies 64 \times 2.0\text{ ms} = \mathbf{128\text{ ms}}$ of pure network latency overhead *per token* ($\le 7.8\text{ tok/s}$ ceiling even with infinite GPU FLOPS).
- **Pipeline Parallelism**: Requires only $k = 3$ point-to-point transfers per token $\implies 3 \times 1.0\text{ ms} = \mathbf{3.0\text{ ms}}$ of network latency overhead per token.

## 3. Decision Outcome

We select **Capability-Weighted Layer-Wise Pipeline Parallelism** built on `llama.cpp` / GGML for `umcf-engine-ai`, augmented with three domain-specific optimizations:

### 3.1 Heterogeneous Layer Slicing & Zero-Copy CAS `mmap`
- GGUF model files stored in `umcf-engine-storage` (CAS) are indexed by tensor byte offsets.
- Each worker downloads and memory-maps (`mmap`) **only the GGUF chunks corresponding to its assigned layer range** `[start_layer..end_layer]` (plus token embeddings on the head node and `output.weight` on the tail node).
- Layer count $\ell_i$ assigned to node $n_i$ is proportional to $\min\left(\text{AllocatableRAM}(n_i), \text{CalibratedTokPerSec}(n_i)\right)$.

### 3.2 Dynamic Activation Quantization (`FP16` vs. `Q8_0`)
- Between pipeline stages, the hidden state tensor has shape `[batch_tokens, hidden_dim]`.
- On high-bandwidth LAN links (`>= 100 Mbps`), activations are transmitted in `FP16` (`8 KiB` per token for `hidden_dim = 4096`).
- On constrained Wi-Fi or WAN links, `umcf-engine-ai` dynamically quantizes intermediate activations to `Q8_0` (`4.25 KiB` per token), halving network transmission time during prompt prefill (`batch_tokens = 512`) from `4.0 MiB` to `2.1 MiB`.

### 3.3 Fault-Tolerant Prompt History Replay for Instant Recovery
- The pipeline head node retains the compact array of generated token IDs (`Vec<u32>`, only `16 KiB` for a 4,096-token conversation).
- If an intermediate mobile worker throttles or disconnects mid-generation, `umcf-scheduler` assigns its layer range `[start_layer..end_layer]` to a standby node andeither streams the local KV-cache slice (if on fast LAN) or replays the token ID array in a single fast prefill batch to reconstruct the missing KV-cache slice without losing user session state.

## 4. Consequences

### Positive
- **Minimal Network Overhead**: Scales cleanly across 2 to 8 consumer devices over standard Wi-Fi without requiring InfiniBand or RoCEv2 RDMA NICs.
- **Partial Model Residency**: A smartphone with only 3 GB of allocatable RAM can meaningfully contribute 8 layers to a 16 GB model without downloading or holding the entire GGUF file.

### Negative & Mitigations
- **Sequential Bubble in Single-Stream Decoding**: During `batch = 1` autoregressive decoding, only one pipeline stage computes at an instant unless multiple concurrent requests or speculative decoding are active. *Mitigation*: Support micro-batched multi-request pipelining and optional draft-model **Speculative Decoding** on the tail/head node so each pipeline pass verifies 4–6 candidate tokens in parallel.
