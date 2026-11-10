---
name: umcf-distributed-systems
description: >-
  Guides the design and implementation of UMCF distributed systems components, including heterogeneous task scheduling, pipeline-parallel llama.cpp inference sharding, dynamic fault recovery, backpressure, and state coordination. Use when working on umcf-scheduler, umcf-llama-engine, or control-plane Durable Objects.
---

# UMCF Distributed Systems & Heterogeneous Scheduling Skill

Use this skill when designing or implementing distributed scheduling, model sharding, fault tolerance, or cluster coordination across mobile, desktop, and server nodes.

---

## 1. Honest Heterogeneous Resource Model

Mobile and desktop nodes differ wildly in compute throughput, memory bandwidth, battery constraints, and network stability. All distributed algorithms in UMCF must model each node $i$ as a tuple of dynamic, probed metrics:

$$\text{Node}_i = \left(\text{RAM}_{\text{usable}, i}, \text{VRAM}_{\text{usable}, i}, \text{Compute}_{\text{TPS}, i}, \text{PowerPolicy}_i, \text{ThermalState}_i, \text{Link}_{i, j}(\text{RTT}, \text{BW}, \text{Loss})\right)$$

1. **No Shared-Memory Illusion**:
   - Never assume transparent remote memory paging. Accessing bytes on another node requires explicit message framing, serialization, encryption, and network transmission over QUIC.
2. **Locality-First Principle**:
   - If a workload (e.g., a 3B Q4_K_M model requiring 2.2 GB RAM) fits comfortably within a single node's usable memory watermark and local compute meets the latency SLO, **schedule it locally on that single node** rather than incurring cross-network activation latency.
   - Use multi-node sharding when:
     - Total model memory > single-node usable RAM/VRAM, **or**
     - Data-parallel / batch requests can saturate multiple nodes concurrently, **or**
     - Local node is thermally/battery constrained and offloads execution to a plugged-in desktop/server peer.

---

## 2. Distributed `llama.cpp` Pipeline Partitioning Algorithm

When partitioning an $L$-layer transformer model across $K$ selected nodes $(n_0, n_1, \dots, n_{K-1})$:

1. **Ring / Pipeline Ordering**:
   - Order selected nodes to minimize total ring/pipeline hop RTT:
     $$\min_{\pi} \sum_{k=0}^{K-1} \text{RTT}(\pi(k), \pi((k+1) \bmod K))$$
2. **Memory-Proportional & Compute-Balanced Layer Assignment**:
   - Each node $k$ must satisfy:
     $$\text{Mem}_{\text{layers}}(l_{\text{start}, k} .. l_{\text{end}, k}) + \text{Mem}_{\text{KV}}(l_{\text{start}, k} .. l_{\text{end}, k}) + \text{Mem}_{\text{scratch}} \le \alpha \cdot \text{RAM}_{\text{usable}, k}$$
     where $\alpha \in [0.70, 0.85]$ is the safety watermark factor.
   - Subject to memory feasibility, assign layer counts proportional to each node's calibrated compute throughput ($\text{FLOP/s}$ or probed GGUF layer eval benchmark) so stage execution times are balanced.
3. **Activation Streaming & Micro-Batching**:
   - During prompt prefill (`n_tokens > 1`), split long prompts into micro-batches (e.g., 64–256 tokens) so Stage $k$ computes chunk $m+1$ while Stage $k+1$ computes chunk $m$ (pipeline parallelism).
   - Compress or quantize inter-stage activation tensors (FP16 or Q8_0) with CRC32C/BLAKE3 frame integrity checks.

---

## 3. Fault Tolerance, Leases & Mid-Inference Recovery

Mobile nodes frequently experience network handoffs or thermal throttling:

1. **Stage Heartbeats & Leases**:
   - Every active pipeline stage maintains a fast QUIC keep-alive (e.g., 1s interval, 3s timeout) and publishes incremental thermal/memory pressure flags inside activation frame headers.
2. **Preemptive Live Migration (Thermal / Battery Warning)**:
   - When a node detects approaching thermal throttling (`ThermalState::Serious`) or battery floor (`Battery <= min_floor + 2%`), it sets the `DRAIN_REQUESTED` flag in its response frame.
   - The coordinator selects a standby peer, pre-warms the assigned GGUF layer range on the standby peer, transfers or recomputes the prompt KV state, and switches routing at the next token boundary without dropping the user session.
3. **Hard Disconnect Recovery**:
   - If a worker node drops abruptly during generation, the coordinator catches `PipelineStageTimeout`, marks the peer `Unreachable`, reassigns the missing layer range to an eligible peer (or falls back to a smaller model if cluster memory is insufficient), replays the current token context to rebuild that stage's KV cache, and resumes streaming tokens.
