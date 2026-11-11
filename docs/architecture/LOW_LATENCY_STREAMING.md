# LOW_LATENCY_STREAMING.md — Ultra-Low-Latency Remote Application & Game Streaming

This document defines the architectural specifications, wire protocols, latency budgets, and security invariants for the **Universal Mobile Compute Fabric (UMCF)** remote desktop and game streaming subsystem (`umcf-stream`).

---

## 1. Zero-Copy Display Capture & Hardware Encoding Host (`S6.P3.SP1`)

Console-grade interactive remote desktop and game streaming requires end-to-end glass-to-glass latency below $30\text{ ms}$ over LAN and below $60\text{ ms}$ over WAN. To achieve this, the host pipeline eliminates frame buffering delay and minimizes capture-to-encode latency to $\le 6.0\text{ ms}$ at 1080p60.

```
+---------------------------------------------------------------------------------------------------------+
|                                              HOST PIPELINE                                              |
|                                                                                                         |
|  [Desktop / Game Surface]                                                                               |
|            |                                                                                            |
|            v                                                                                            |
|  [Zero-Copy GPU Capture] (DXGI Output Duplication / PipeWire DMA-BUF / ScreenCaptureKit)                |
|            |  - Shared D3D11/D3D12/Metal GPU Texture                                                    |
|            |  - Hardware Cursor Metadata Extraction (pos + shape hash)                                  |
|            v                                                                                            |
|  [Sub-Millisecond Color Conversion] (Compute Shader: `rgb_to_nv12.wgsl`)                                |
|            |  - RGBA/BGRA -> NV12 (BT.709 8-bit) or P010 (BT.2020 PQ 10-bit HDR)                        |
|            |  - < 0.5 ms execution overhead on GPU                                                      |
|            v                                                                                            |
|  [Ultra-Low-Latency Hardware Encoder] (ZeroLatencyVideoEncoder: NVENC / QSV / AMF / VideoToolbox / VAAPI)  |
|            |  - max_b_frames = 0, rc_lookahead = 0 (Zero Lookahead / Zero B-Frames)                     |
|            |  - Single-Frame VBV Buffer: `bitrate / fps * 1.2` (Zero Queuing Delay)                     |
|            |  - Periodic Intra-Refresh (GDR) + Atomic On-Demand IDR Keyframe Injection                  |
|            v                                                                                            |
|  [Low-Delay Audio Loopback] (LoopbackAudioCapturer: 48 kHz Stereo -> 5ms Opus with in-band FEC)         |
+---------------------------------------------------------------------------------------------------------+
```

### 1.1 Cross-Platform Display & Window Capture Engine (`DisplayCapturer`)
1. **Windows Platform Backend**:
   - Uses the DXGI Desktop Duplication API (`IDXGIOutputDuplication`) and `Windows.Graphics.Capture`.
   - Accesses D3D11 / D3D12 shared surfaces directly in GPU VRAM without staging copies through CPU system RAM.
   - Extracts hardware cursor position, visibility, and shape hashes alongside dirty-rectangle coordinates.
2. **Linux Platform Backend**:
   - Uses PipeWire (`xdg-desktop-portal` ScreenCast) with DMA-BUF file descriptor passing, enabling direct GPU import into Vulkan/VAAPI contexts with zero CPU memory copies.
   - Provides X11 `XShmGetImage` fallback for legacy environments.
3. **macOS Platform Backend**:
   - Uses `ScreenCaptureKit` (`SCStream`) delivering GPU-backed `IOSurface` references directly into Metal contexts.
4. **Headless / CI Verification Backend (`HeadlessSyntheticCapturer`)**:
   - Generates deterministic test patterns with embedded 64-bit frame-counter barcodes and microsecond timestamps for automated CI regression testing and latency verification.

### 1.2 Sub-Millisecond GPU Color Space Conversion
- Video hardware encoders require planar or semi-planar YUV input surfaces (NV12 for 8-bit SDR, P010 for 10-bit HDR), whereas desktop window managers render in 32-bit RGBA or BGRA.
- `ColorConverter` executes color conversion via compute shaders (`rgb_to_nv12.wgsl`) in $< 0.5\text{ ms}$:
  - **BT.709 NV12 Conversion**:
    $$Y = 0.2126 \cdot R + 0.7152 \cdot G + 0.0722 \cdot B$$
    $$U = -0.1146 \cdot R - 0.3854 \cdot G + 0.5000 \cdot B + 128$$
    $$V = 0.5000 \cdot R - 0.4542 \cdot G - 0.0458 \cdot B + 128$$
  - **BT.2020 PQ P010 (10-bit HDR)**:
    Quantizes color channels into 10-bit precision stored in the upper 10 bits of 16-bit words (`u16 << 6`).

### 1.3 Ultra-Low-Latency Encoder Configuration & Tuning
- **Zero Lookahead & Zero B-Frames**:
  - In interactive game and desktop streaming, B-frames introduce bidirectional reordering delay ($N \times \text{frame\_time}$), increasing glass-to-glass latency by $16..50\text{ ms}$.
  - `LowLatencyEncoderConfig` strictly enforces `max_b_frames = 0`, `rc_lookahead = 0`, and `num_reorder_frames = 0`.
- **Single-Frame VBV Buffer Sizing**:
  - Real-time video must never spike the network socket with multi-frame bursts.
  - VBV buffer size is tuned to a single frame duration ($1.2 \times \frac{\text{bitrate}}{\text{fps}}$), ensuring rate control constrains each compressed frame within the single-frame transmission window.
- **Atomic On-Demand IDR & Gradual Decoder Refresh (GDR)**:
  - When the transport layer reports packet loss (PLI / FIR), `force_idr_on_next_frame()` atomically triggers the immediate emission of a standalone IDR keyframe with in-band parameter sets (`SPS`/`PPS`/`VPS`).
  - Optional Periodic Gradual Decoder Refresh (GDR) refreshes the frame column-by-column, eliminating large bitrate spikes associated with full-frame I-frames.

### 1.4 Low-Latency System Audio Loopback
- `LoopbackAudioCapturer` captures desktop audio at $48\text{ kHz}$ stereo.
- Packetizes audio into $5\text{ ms}$ ($240$ samples per channel) or $10\text{ ms}$ ($480$ samples per channel) frames.
- Encodes with `OPUS_APPLICATION_RESTRICTED_LOWDELAY` and in-band Forward Error Correction (FEC), guaranteeing sub-$5\text{ ms}$ audio encode latency and seamless concealment of single-packet network drops.
