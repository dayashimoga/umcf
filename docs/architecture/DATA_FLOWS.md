# Universal Mobile Compute Fabric (UMCF) — End-to-End Data Flows

> [!NOTE]
> This document details the six canonical runtime data flows of UMCF, including protocol sequence diagrams, wire packet structures, cryptographic handshakes, and failure recovery transitions.

---

## Flow 1: Device Identity Generation, QR Code Enrollment & Mutual Attestation

### 1.1 Overview
When a new device boots UMCF for the first time, it generates an unexportable Ed25519 identity keypair in the platform keystore. To join an existing user fabric without typing passwords or exposing open ports, the existing Master/Coordinator displays a short-lived QR code (or shareable URI `umcf://enroll?...`) containing the cluster's root public key, one-time cryptographic nonce, LAN/WAN endpoints, and expiration timestamp.

### 1.2 Sequence Diagram

```mermaid
sequenceDiagram
    autonumber
    participant Keystore as OS Secure Keystore
    participant Joiner as New Node (Android/Desktop)
    participant Master as Cluster Master / Coordinator
    participant Control as Control Plane (CF DO / LAN)

    Joiner->>Keystore: Generate Ed25519 Keypair on First Boot
    Keystore-->>Joiner: PublicKey_J, NodeId_J = "umcf1_" + base32(blake3(PubKey_J))
    Master->>Control: Request Enrollment Token (TTL = 120s, Scope = Worker/Hybrid)
    Control-->>Master: EnrollmentTicket { cluster_id, nonce, master_pubkey, sig_M }
    Master->>Master: Render QR Code (umcf://enroll?c=cluster_id&n=nonce&pk=master_pubkey&lan=ip:port&sig=sig_M)
    
    Joiner->>Master: Scan QR Code / Import URI
    Joiner->>Joiner: Verify sig_M using master_pubkey & check timestamp < TTL
    Joiner->>Control: POST /v1/clusters/{id}/enroll (or Direct LAN QUIC Hello)
    Note over Joiner,Control: Payload: { NodeId_J, PubKey_J, ResourceProfile, Sign_J(nonce || cluster_id || PubKey_M) }
    
    Control->>Control: Verify Sign_J & Single-Use Nonce Validity
    Control->>Master: Notify New Peer Attestation Validated
    Master->>Joiner: Issue Cluster Membership Certificate & RBAC Token (Signed by Cluster Root)
    Joiner->>Keystore: Persist Cluster Root PubKey & Membership Certificate
    Joiner->>Master: Establish mTLS QUIC Data Plane (RFC 7250 Raw Public Key Pinned)
```

### 1.3 Enrollment Payload Schema
```json
{
  "version": 1,
  "cluster_id": "clu_01938a7f9c207b8291a4",
  "enrollment_nonce": "e4f901a28b7c6d5e4f3a2b1c0d9e8f7a",
  "master_node_id": "umcf1_7q9x2m4k8p1w3v5n6b8c0d2f4g6h8j0k",
  "master_pubkey_ed25519": "b4f2...9a01",
  "lan_candidates": ["192.168.1.42:4433", "[fe80::1a2b:3c4d]:4433"],
  "wan_control_url": "https://signal.umcf.internal/v1/clusters/clu_01938a7f9c207b8291a4",
  "expires_at_unix_ms": 1762756800000,
  "master_signature": "8d1c...3e7f"
}
```

---

## Flow 2: LAN mDNS Discovery vs. WAN STUN/ICE Hole-Punching & Relay Fallback

### 2.1 Overview
UMCF prioritizes lowest-latency direct connectivity. Every node simultaneously runs a LAN zero-conf discovery loop (`_umcf._udp.local` on port `5353`) and a WAN ICE/STUN candidate gathering state machine coordinated through the Control Plane.

### 2.2 Sequence Diagram

```mermaid
sequenceDiagram
    autonumber
    participant NodeA as Node A (Peer Initiator)
    participant mDNS as LAN Multicast (_umcf._udp.local)
    participant Signal as Control Plane (CF Durable Object)
    participant STUN as STUN / TURN / Relay Node
    participant NodeB as Node B (Target Peer)

    par Fast Path: Same LAN Subnet
        NodeA->>mDNS: Multicast Query (_umcf._udp.local, TXT: cluster_id_hash)
        NodeB-->>mDNS: Multicast Response (A/AAAA: 192.168.1.55:4433, NodeId_B)
        NodeA->>NodeB: Direct QUIC Handshake (192.168.1.55:4433, mTLS Ed25519)
        NodeB-->>NodeA: QUIC 1-RTT Established (RTT < 2ms)
    and WAN Path: Disjoint Networks / NAT Traversal
        NodeA->>STUN: STUN Binding Request (UDP :4433)
        STUN-->>NodeA: XOR-MAPPED-ADDRESS (203.0.113.10:51820, NAT Type: RestrictedCone)
        NodeB->>STUN: STUN Binding Request (UDP :4433)
        STUN-->>NodeB: XOR-MAPPED-ADDRESS (198.51.100.22:60112, NAT Type: Symmetric)
        
        NodeA->>Signal: Publish ICE Candidates [LAN, ServerReflexive, Relay]
        NodeB->>Signal: Publish ICE Candidates [LAN, ServerReflexive, Relay]
        Signal-->>NodeA: Deliver Node B Candidates
        Signal-->>NodeB: Deliver Node A Candidates
        
        NodeA->>NodeB: Simultaneous UDP Hole-Punch Probes (QUIC Initial Packets, 600ms budget)
        NodeB->>NodeA: Simultaneous UDP Hole-Punch Probes (QUIC Initial Packets, 600ms budget)
        
        alt Hole-Punch Succeeds
            NodeA<-->NodeB: Direct P2P WAN QUIC Established (Zero Relay Overhead)
        else Symmetric NAT / UDP Blocked Firewall (> 600ms Timeout)
            NodeA->>STUN: Allocate Encrypted QUIC-in-QUIC Relay Circuit (DERP-style)
            NodeB->>STUN: Bind to Relay Circuit ID
            NodeA<-->STUN: End-to-End Encrypted mTLS QUIC Frames
            STUN<-->NodeB: Forwarded Opaque QUIC Datagrams/Streams
        end
    end
```

### 2.3 Path Upgrade Invariant
Even if a session starts on the **Relay Fallback** path to guarantee immediate connectivity (within `800ms`), `umcf-core` continues periodic background UDP hole-punch probes every `30s` and transparently migrates the active QUIC connection (`quinn` Connection Migration) to a direct P2P path if network conditions permit.

---

## Flow 3: Hardware Discovery, Calibration & Signed Benchmark Attestation

### 3.1 Overview
Reported hardware specifications (e.g., "8 Core CPU" or "Adreno GPU") are insufficient for accurate DAG scheduling because mobile SoCs vary wildly in sustained memory bandwidth and thermal throttling behavior. Upon joining or hardware change, `umcf-worker` executes a fast (`< 1,500ms`) deterministic micro-benchmark suite and signs the resulting capability profile.

### 3.2 Sequence Diagram

```mermaid
sequenceDiagram
    autonumber
    participant Worker as umcf-worker
    participant HW as Local CPU / GPU / NPU Backends
    participant CAS as Local Profile Cache
    participant Coord as umcf-coordinator (Master)
    participant Sched as umcf-scheduler

    Worker->>HW: Probe OS & Backend APIs (sysinfo, Vulkan, Metal, CUDA, WebGPU)
    HW-->>Worker: Raw Device Enumerations (RAM, VRAM, Compute Units, Thermal Sensors)
    
    Worker->>CAS: Check Cached Calibration for (HardwareFingerprint, DriverVersion)
    alt Cache Miss or Expired (> 7 Days)
        Worker->>HW: Run 300ms Memory Copy Triad Benchmark (Measure Sustained GB/s)
        HW-->>Worker: measured_mem_bw_gbps = 68.4 GB/s
        Worker->>HW: Run 500ms GEMM / Q4_K_M MatMul Micro-Kernel (1024x1024)
        HW-->>Worker: fp16_tflops = 4.2, int4_tok_s_equiv = 38.5
        Worker->>CAS: Sign & Persist CalibrationAttestation
    end
    
    Worker->>Coord: Send CapabilityReport (ResourceProfile + Signed Calibration)
    Coord->>Coord: Verify Signature & Sanity-Check Bounds Against Hardware Class
    Coord->>Sched: Update ClusterSnapshot & Recompute Link/Node Weight Matrix
```

---

## Flow 4: Distributed AI Inference (Layer-Partitioned Pipeline Parallelism)

### 4.1 Overview
This flow illustrates executing a `Llama-3-8B-Instruct` (32 layers, `Q4_K_M` ~4.9 GB) across three nodes where no single mobile node wants to dedicate 5 GB of RAM, or where combining a desktop GPU with two laptops accelerates prefill and token generation:
- **Node 1 (Coordinator + Worker 1 — Laptop)**: Token Embedding + Layers `0..10`
- **Node 2 (Worker 2 — High-End Android Tablet)**: Layers `11..20`
- **Node 3 (Worker 3 — Desktop GPU)**: Layers `21..31` + RMSNorm + Output LM Head + Sampler

### 4.2 Sequence Diagram

```mermaid
sequenceDiagram
    autonumber
    participant Client as Client UI / API
    participant Coord as Master (umcf-scheduler)
    participant W1 as Worker 1 (Layers 0..10)
    participant W2 as Worker 2 (Layers 11..20)
    participant W3 as Worker 3 (Layers 21..31 + Head)

    Client->>Coord: Submit Inference Request (model="llama3-8b-q4km", prompt="Explain QUIC...")
    Coord->>Coord: Solve Pipeline Placement (Assign L0..10 -> W1, L11..20 -> W2, L21..31 -> W3)
    
    par Stage Model Layer Slices from CAS
        Coord->>W1: TaskLeaseAssign (Layers 0..10, session_id=S1)
        Coord->>W2: TaskLeaseAssign (Layers 11..20, session_id=S1)
        Coord->>W3: TaskLeaseAssign (Layers 21..31 + Head, session_id=S1)
    end
    W1-->>Coord: LeaseReady (mmap complete, KV-Cache allocated for L0..10)
    W2-->>Coord: LeaseReady (mmap complete, KV-Cache allocated for L11..20)
    W3-->>Coord: LeaseReady (mmap complete, KV-Cache allocated for L21..31)

    loop Autoregressive Token Generation (Prefill + Decode Steps)
        Client->>W1: Input Token ID(s) [pos=0..N]
        W1->>W1: Compute Embedding + Transformer Layers 0..10 (Update local KV-Cache L0..10)
        W1->>W2: QUIC Stream TensorRpcForward { session=S1, pos, shape=[1, seq, 4096], fp16_bytes }
        W2->>W2: Compute Transformer Layers 11..20 (Update local KV-Cache L11..20)
        W2->>W3: QUIC Stream TensorRpcForward { session=S1, pos, shape=[1, seq, 4096], fp16_bytes }
        W3->>W3: Compute Layers 21..31 + Norm + Logits + Sample Next Token (token_id)
        
        par Stream Token to User & Feed Back to Pipeline Head
            W3-->>Client: Stream Token Event (token_id, text_piece, latency_ms)
            W3->>W1: Next Step Token ID { session=S1, pos=N+1, token_id } (4-byte fast datagram/stream)
        end
    end
```

### 4.3 `TensorRpcForward` Wire Packet Structure
| Offset | Field | Type | Size | Description |
| :--- | :--- | :--- | :--- | :--- |
| `0..16` | `SessionId` | `Uuid` | 16 B | Identifies the active inference KV-cache context |
| `16..20` | `SequencePos` | `u32` | 4 B | Starting token position in the KV-cache (`0..context_len`) |
| `20..24` | `BatchTokens` | `u32` | 4 B | Number of tokens in this micro-batch (`>1` for prefill, `1` for decode) |
| `24..26` | `StartLayer` | `u16` | 2 B | Next target transformer layer index (e.g., `11`) |
| `26..27` | `DType` | `u8` | 1 B | `0x01 = FP16`, `0x02 = BF16`, `0x03 = Q8_0` (compressed activation) |
| `27..28` | `Flags` | `u8` | 1 B | `bit 0: EOS`, `bit 1: KV_RESET`, `bit 2: CHECKPOINT_REQ` |
| `28..32` | `HiddenDim` | `u32` | 4 B | Model embedding dimension $d_{\text{model}}$ (e.g., `4096`) |
| `32..N` | `TensorData` | `[u8]` | `Batch * HiddenDim * sizeof(DType)` | Raw aligned tensor buffer (`8 KiB` for `FP16` or `4 KiB` for `Q8_0` at `Batch=1`) |

> [!TIP]
> **Activation Quantization Optimization**: During WAN or congested Wi-Fi operation, UMCF dynamically switches `DType` from `FP16` (`8 KiB`/token) to `Q8_0` (`4.25 KiB`/token), cutting inter-node bandwidth in half with `< 0.02` perplexity impact.

---

## Flow 5: Low-Latency Remote Application / Game Streaming & Input Return Channel

### 5.1 Sequence Diagram

```mermaid
sequenceDiagram
    autonumber
    participant Viewer as Client Node (Mobile/Web/Laptop)
    participant Host as Host Worker (Desktop dGPU)
    participant Capture as OS Capture (DXGI / PipeWire / ScreenCaptureKit)
    participant Encoder as HW Encoder (NVENC / AMF / VideoToolbox)

    Viewer->>Host: Open StreamSession (1080p@120fps, HEVC, Bitrate=25Mbps, LowLatency=true)
    Host->>Capture: Acquire GPU Shared Texture Handle (Zero-Copy VRAM)
    Host-->>Viewer: StreamSessionReady (QUIC Datagram FlowID=42, Audio FlowID=43, Input StreamID=4)

    par High-Frequency Input Return Channel (120Hz..500Hz)
        Viewer->>Host: QUIC Stream/Datagram InputEventBatch (MouseDelta dx,dy, KeyCode, GamepadAxis)
        Host->>Host: Verify Input Capability Scope & Inject into OS Input Queue (< 1ms)
    and Ultra-Low-Latency Video Pipeline (< 16.6ms Glass-to-Glass LAN)
        Capture->>Encoder: New Frame VSync Ready (VRAM DMA-BUF / D3D11Texture2D)
        Encoder->>Encoder: Single-Pass Intra-Refresh Encode (No B-Frames, Slice-Based, ~2.5ms)
        Encoder->>Viewer: Transmit Packetized NAL Units + Reed-Solomon FEC over QUIC Datagrams
        Viewer->>Viewer: Hardware Decode (MediaCodec / WebCodecs) & Render Immediately on Surface
    end

    Viewer->>Host: Periodic RTCP/QUIC Feedback (PacketLoss=0.2%, DecodeTime=1.8ms, PlayoutDelay=0ms)
    Host->>Encoder: Dynamic Bitrate & FEC Adjustment (AIMD / GCC Congestion Control)
```

---

## Flow 6: Worker Mid-Task Preemption, Thermal Throttling & Checkpoint Recovery

### 6.1 Overview
Mobile devices experience rapid thermal accumulation or sudden user preemption (unplugging charger, opening a foreground camera app). UMCF handles worker degradation gracefully without aborting the user's multi-node inference or media job.

### 6.2 Sequence Diagram

```mermaid
sequenceDiagram
    autonumber
    participant Sensor as OS Thermal / Battery Monitor
    participant W2 as Degrading Worker (Node 2: Layers 11..20)
    participant Coord as Master Coordinator (umcf-scheduler)
    participant W4 as Standby / Replacement Worker (Node 4)
    participant W1 as Upstream Worker (Node 1: Layers 0..10)
    participant W3 as Downstream Worker (Node 3: Layers 21..31)

    Sensor->>W2: Thermal Callback: BatteryTemp = 43.2C (ThermalState::Critical)
    W2->>W2: Transition State -> NodeRole::Draining
    W2->>Coord: Priority Alert: TaskPreemptNotice { lease_id, reason: ThermalCritical, drain_deadline_ms: 2000 }
    
    Coord->>Coord: Run IncrementalRebalance(degraded_node = W2)
    Coord->>W4: TaskLeaseAssign (Layers 11..20, session_id=S1, Urgent Warmup)
    W4->>W4: mmap CAS Weights for Layers 11..20 (Already cached or swarm-fetched)
    
    par KV-Cache Migration vs. Fast Re-Prefill Decision
        alt KV-Cache Size < 16 MiB and W2->W4 LAN Link >= 50 Mbps
            Coord->>W2: Command: MigrateKvCache(target = W4, session_id = S1)
            W2->>W4: Direct QUIC Stream KvCacheSync (Compressed FP16/Q8 KV Tensors for L11..20)
            W4-->>Coord: KvCacheRestored(pos = current_pos)
        else WAN Link or Faster to Re-Prefill Prompt Token History
            Coord->>W1: Replay Token History Prefill Batch [0..current_pos] through W1 -> W4
            W1->>W4: Batched Prefill Activations (Layers 11..20 reconstruct KV-Cache in < 300ms)
            W4-->>Coord: KvCacheReconstructed(pos = current_pos)
        end
    end

    Coord->>W1: Update Pipeline Routing Table (NextHop for L11 = W4, lease_epoch + 1)
    Coord->>W4: Update Pipeline Routing Table (NextHop for L21 = W3, lease_epoch + 1)
    Coord->>W2: Revoke Lease & Authorize Transition to ThrottledStandby
    W2->>W2: Free KV-Cache & Unmap Weights (Cool-down Hysteresis until Temp <= 39.0C)
```
