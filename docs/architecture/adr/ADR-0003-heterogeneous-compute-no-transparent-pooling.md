# ADR-0003: Rejection of Transparent Shared-Memory Pooling in Favor of Capability-Negotiated Partitioning

- **Status**: Accepted
- **Date**: 2026-11-10
- **Deciders**: UMCF Core Architecture Team
- **Related Components**: `umcf-scheduler`, `umcf-engine-ai`, `umcf-engine-media`, `umcf-core`

---

## 1. Context and Problem Statement

Users and stakeholders often envision combining multiple consumer devices (e.g., three smartphones with 8 GB RAM each and one laptop with 16 GB RAM) into a single "virtual 40 GB unified GPU/CPU machine" capable of running arbitrary unmodified binaries or monolithic GPU kernels as if all memory were local.

We must decide whether UMCF should attempt **transparent Distributed Shared Memory (DSM) / virtualized GPU API interception** or require **explicit capability-negotiated pipeline, tensor, and task partitioning**.

## 2. Technical Analysis of Why Transparent Pooling Fails

1. **Latency & Bandwidth Physics ($10,000\times$ Penalty)**:
   - Local GPU VRAM / Unified SoC LPDDR5X operates at **$200\text{--}1,000\text{ GB/s}$** with **$\sim 50\text{--}100\text{ ns}$** latency.
   - Consumer Wi-Fi 6 / Gigabit LAN operates at **$0.05\text{--}0.12\text{ GB/s}$** with **$1,000,000\text{--}5,000,000\text{ ns}$ ($1\text{--}5\text{ ms}$)** latency.
   - If a GPU shader or CPU thread faults on a remote memory page over Wi-Fi, the compute pipeline stalls for millions of cycles. Workloads with frequent random memory access slow down by $100\times\text{--}1,000\times$, making distributed execution vastly slower than running with swap on a single device.
2. **Heterogeneous ISA & Memory Formats**:
   - An Android phone with a Qualcomm Hexagon NPU (`QNN`), a MacBook with Apple Silicon (`Metal`), and a Windows PC with an NVIDIA GPU (`CUDA`) do not share instruction sets, page table formats, floating-point rounding modes, or tensor memory layouts.
   - Low-level GPU command buffers (e.g., CUDA PTX or Vulkan SPIR-V pointers) cannot be transparently split mid-kernel across two incompatible GPUs over a network socket.

## 3. Decision Outcome

**UMCF explicitly rejects transparent shared-memory hardware pooling and virtualized single-image OS/GPU abstraction.**

Instead, UMCF adopts **Capability-Negotiated DAG Partitioning** governed by `umcf-scheduler`:

1. **Explicit Stage Boundaries**: Workloads are decomposed at coarse, mathematically well-defined boundaries where the **Compute-to-Communication Ratio** is high:
   - **LLM Inference (`umcf-engine-ai`)**: Partitioned by contiguous **Transformer Layer Ranges** ($L_i \dots L_j$). Each node holds its own layers' weights and KV-cache in local native memory (`CUDA`, `Metal`, `Vulkan`, `CPU`), transmitting only compact intermediate activation tensors ($\sim 4\text{--}8\text{ KiB}$ per token) over QUIC.
   - **Media Transcoding & Rendering (`umcf-engine-media`)**: Partitioned by independent **Group-of-Pictures (GOP)** video segments or frame/tile ranges.
   - **Batch Compute / Embeddings**: Partitioned by **Data Shards / Micro-Batches**.
   - **Interactive Apps & Games (`umcf-engine-stream`)**: Executed on the single most capable host GPU and streamed via ultra-low-latency hardware video encoding (`< 16.6ms` LAN) to thin client nodes.
2. **Single-Node Colocation Preference Rule**: If a single node in the cluster possesses sufficient RAM/VRAM and compute FLOPS to meet the job's latency SLO alone, `umcf-scheduler` **must** schedule the job on that single node rather than artificially splitting it across slower network links.

## 4. Consequences

### Positive
- **Physically Achievable Speedups & Capacity Expansion**: Enables running 30B–70B parameter LLMs across multiple consumer devices whose combined RAM exceeds any single device, while keeping network traffic under $1\text{ MB/s}$ during token decoding.
- **True Cross-Vendor Interoperability**: An Apple Metal MacBook, an NVIDIA CUDA Windows PC, and a Vulkan Android phone can seamlessly collaborate on the same LLM inference pipeline because they exchange standard `FP16`/`Q8_0` activation tensors over `umcf-proto` rather than proprietary GPU memory pointers.

### Negative & Mitigations
- **Workloads Must Map to Supported Engine Archetypes**: Arbitrary closed-source Windows `.exe` games cannot have their internal DirectX draw calls split across two phones; instead, they are supported via the **Remote Desktop & Game Streaming Engine (`umcf-engine-stream`)** hosted on a capable desktop node.
