# GPU Compute, Distributed Rendering & Media Streaming Architecture (Stage 6)

## 1. Cross-Platform GPU Compute & Shader Sandboxing

### 1.1 WGPU/Vulkan/Metal Compute Shader Sandbox & Buffer Validation (`S6.P1.SP1`)

Because untrusted or distributed cluster nodes execute compute workloads across heterogeneous devices, UMCF enforces strict memory isolation, zero uninitialized VRAM leakage, and driver hang prevention at the shader and buffer level.

#### 1.1.1 Naga AST Loop-Fuel Watchdog Injection
- **Threat Model**: Malicious, buggy, or poorly optimized compute shaders containing unbounded loops (`loop { ... }` or `while (true)`) can hang physical GPU command queues, causing desktop/mobile OS GPU TDR (Timeout Detection and Recovery) driver resets or mobile app termination.
- **Enforcement**:
  - The AST validator traverses incoming WGSL or SPIR-V modules via `naga`.
  - When loop constructs are encountered, an AST transformation injects a fuel counter initialized to `100,000u`.
  - On each loop iteration, the fuel counter decrements. If it reaches zero, a `break` statement executes immediately, setting an atomic diagnostic flag `ShaderLoopFuelExhausted`.
  - Submissions are guaranteed to terminate within $\le 150\text{ ms}$ without resetting the host OS GPU driver.

#### 1.1.2 Zero-Leak VRAM Buffer Allocation (`BoundedVramAllocator`)
- **Residual Memory Threat**: Compute shaders that bind uninitialized storage buffers can potentially read sensitive memory bytes (such as decrypted model activations, frames from prior remote desktop sessions, or secret keys) previously allocated in VRAM by other workloads.
- **Mitigation**:
  - `BoundedVramAllocator` enforces a hard quota ceiling (e.g. 50% free VRAM on mobile, 80% on desktop).
  - Every buffer allocation and recycled slot undergoes mandatory cryptographic zero-scrubbing (`clear_buffer(0x00)`).
  - Canary tests confirm that buffers intentionally populated with `0xDEADBEEF` are wiped completely to `0x00000000` before reuse.

#### 1.1.3 Bounds Check Policy (`ReadZeroSkipWrite`)
- Storage, uniform, and texture access bounds are validated using Naga's `ReadZeroSkipWrite` policy.
- Out-of-bounds reads safely evaluate to `0.0` / `0u`.
- Out-of-bounds writes are discarded cleanly without triggering memory fault exceptions or memory corruption across adjacent GPU buffers.

#### 1.1.4 Time-Sliced Compute Dispatches
- Large 3D workgroup dispatches (e.g., $65,536$ workgroups) are partitioned into time-sliced chunks ($\le 1,024$ workgroups per chunk).
- Guarantees each command submission takes $\le 50\text{ ms}$, ensuring mobile UI compositors never drop frames and allowing thermal or cancellation preemption between slices.

---

### 1.2 Adaptive Tile/Frame Decomposition & Progressive Compositor (`S6.P1.SP2`)

High-resolution distributed rendering (e.g., 4K/8K path tracing or raymarching) requires spatial and temporal decomposition that adapts to heterogeneous device capabilities, eliminates straggler tail latency, and provides zero-seam progressive compositing.

#### 1.2.1 Space-Filling Curve & Center-Spiral Tile Ordering
- **Spatial Grid Decomposition**:
  - Frames of arbitrary dimensions $(W, H)$ are subdivided into tiles $(T_W, T_H)$ (default $128 \times 128$).
  - Boundary tiles along right and bottom edges are dynamically clamped to exact pixel boundaries (e.g., $1080 = 8 \times 128 + 56$, bottom tiles are $128 \times 56$), eliminating memory over-allocation and padding seams.
- **Center-Spiral Priority**:
  - Tiles are sorted in ascending order of Euclidean distance from the frame center:
    $$D_{\text{center}} = \sqrt{(X_{\text{tile}} + W_{\text{tile}}/2 - W/2)^2 + (Y_{\text{tile}} + H_{\text{tile}}/2 - H/2)^2}$$
  - Center focal regions are dispatched and rendered first, providing immediate visual feedback to interactive users before peripheral tiles complete.
  - Morton Z-order curve indices (`morton_2d`) are embedded in every `TileDescriptor` for spatial cache coherence.

#### 1.2.2 Adaptive Batch Sizing by Worker Throughput EWMA
- Worker nodes maintain an exponentially weighted moving average (EWMA, $\alpha = 0.25$) of rendering throughput in `pixels_per_ms`.
- High-TFLOPS desktop GPUs (e.g., RTX 4090) receive batches of 4 to 8 tiles to amortize dispatch and network latency.
- Mobile GPUs (e.g., Tensor, Apple A-series) receive single tiles (1 tile per batch) to maintain sub-100 ms responsiveness and avoid queuing stalls.

#### 1.2.3 Tail-Latency Speculative Execution (Straggler Mitigation)
- In distributed environments, mobile or throttled devices can become "stragglers" on final frame tiles.
- When $\ge 95\%$ of frame tiles are complete, the coordinator inspects in-flight tiles.
- If an in-flight tile exceeds $1.5\times$ the fastest node's expected completion time, the coordinator speculatively re-dispatches that tile to an available fast idle worker.
- Whichever worker finishes first commits the tile to the compositor; the redundant completion from the slower node is discarded cleanly without corrupting state (`canceled_redundant_tiles_count += 1`).

#### 1.2.4 Progressive Frame Compositor & Multi-Format Export
- `FrameCompositor` manages a contiguous zero-initialized master frame buffer in `Rgba8UnormSrgb`, `Rgba16Float`, or `Rgba32Float`.
- Each incoming `CompletedTilePacket` blits directly into the frame buffer row-by-row, achieving exact pixel placement and zero boundary seams.
- Emits real-time `ProgressiveFrameDelta` events containing tile bounding box, worker attribution, and BLAKE3 digests for live UI canvas updates.
- Supports lossless export to OpenEXR (`.exr` with IEEE half-float scanlines), raw RGBA buffers, and Netpbm PPM.

---

### 1.3 Deterministic Verification & Checksum Auditing (`S6.P1.SP3`)

In a heterogeneous multi-device compute mesh, integer compute tasks require bit-exact repeatability, whereas floating-point graphics workloads naturally exhibit small ULP (units in the last place) differences across architectures (e.g. FMA contraction and transcendental hardware differences between Vulkan, Metal, and DX12). At the same time, Byzantine cheating nodes or buggy graphics drivers can return corrupt, noisy, or empty black tiles to fraudulently claim compute rewards.

#### 1.3.1 Dual-Mode Verification Policies
- **`DeterministicBitExact`**:
  - Enforced for integer arithmetic, cryptographic operations, and uncompressed bitmap transformations.
  - Requires 100% BLAKE3 hash equality between submitted tiles and redundant evaluations.
- **`FloatingPointPerceptual`**:
  - Enforced for floating-point rendering (e.g. path tracing, raymarching, neural shaders).
  - Tolerates legitimate cross-vendor floating-point ULP drift while strictly flagging visual corruption:
    - $\text{SSIM} \ge 0.998$ (Structural Similarity Index)
    - $\text{PSNR} \ge 48.0\text{ dB}$ (Peak Signal-to-Noise Ratio)
    - $\text{Max Absolute Pixel Difference} \le 0.01$
    - Strict rejection of any `NaN` or `±Inf` values in floating-point color channels (`reject_nan_inf = true`).

#### 1.3.2 3-Stage Verification Pipeline
1. **Stage 1: Zero-Cost Intrinsic Tile Sanity**:
   - Every submitted tile is inspected for exact byte length ($W \times H \times \text{BPP}$), valid header metadata, and the absence of `NaN`/`Inf` floating-point bits.
2. **Stage 2: Secret Canary Micro-Tile Spot-Checking (`CanaryAuditor`)**:
   - The coordinator secretly selects an $8 \times 8$ pixel crop (`SubtileCropBox`) inside a pseudo-random 5% sample of assigned tiles using a CSPRNG keyed by the coordinator's private key.
   - The coordinator computes ground truth only for that tiny $8 \times 8$ crop ($< 0.4\%$ overhead on a $128 \times 128$ tile) and evaluates the crop against the worker's submitted tile using the active `VerificationPolicy`.
3. **Stage 3: Byzantine Quarantine, Slashing Evidence & Automatic Re-Render**:
   - If a worker fails either Stage 1 or Stage 2, the coordinator immediately records signed `VerificationFailureEvidence` (digest, metric discrepancy, timestamps), quarantines and bans the offending worker from the job (`quarantined_workers`), and re-queues all invalidated tiles onto trusted honest nodes.

---

## 2. Distributed Media Transcoding & Processing Pipeline

### 2.1 Hardware Codec Probing & Codec Adapter (`S6.P2.SP1`)

Cluster nodes pool hardware video encoders/decoders (NVIDIA NVENC, Intel QSV, Apple VideoToolbox, Android MediaCodec, AMD AMF, Linux VAAPI, and CPU fallbacks) to process parallel media workloads.

#### 2.1.1 Fast Synthetic Verification (< 40 ms) & Ghost Driver Elimination
- **The Ghost Driver Problem**: Systems frequently list compiled-in encoder flags (e.g. `h264_nvenc` or `hevc_amf`) even when the physical discrete graphics card is absent or disabled (e.g., headless Linux instances, virtual machines, or Intel-only laptops).
- **Mitigation**:
  - `HardwareCodecProber` runs a 5-frame $640 \times 360$ synthetic encode in memory (< 40 ms) before advertising any hardware encoder to the mesh.
  - Candidates that fail driver initialization are discarded as ghost drivers, leaving only operational, verified hardware encoders.
  - Results are cached by OS, driver version, and binary digest.

#### 2.1.2 Structured Argument Sandboxing
- Shell strings (`sh -c` or `cmd /c`) are strictly forbidden across media adapters.
- Arguments are passed as structured arrays with validated paths, rejecting null bytes (`\0`) and path traversal sequences (`..`).

#### 2.1.3 Automatic Hardware-to-Software Parameter Translation
- When hardware session limits are saturated (e.g., NVENC 3-stream consumer limit reached or MediaCodec resource busy), `MediaCodecAdapter` transparently translates hardware rate control parameters to deterministic software equivalents:
  - NVENC/QSV: `-rc vbr -cq <Q> -preset <P>` $\rightarrow$ libx264: `-crf <Q> -preset medium`
  - HEVC QSV: `-global_quality <Q>` $\rightarrow$ libx265: `-crf <Q> -preset medium`
  - AV1 QSV: `-cq <Q>` $\rightarrow$ libsvtav1: `-crf <Q> -preset 6`
- Preserves target resolution, pixel format (e.g., `Nv12`, `P010le`), and color metadata without causing transcode job aborts.

#### 2.1.4 Sub-50 ms Cancellation Teardown
- Asynchronous task cancellation instantly signals child processes and media contexts via atomic cancellation flags and OS process signals (`SIGTERM` / `TerminateProcess`), cleaning up intermediate scratch files in $< 50\text{ ms}$ with zero zombie processes.

### 2.2 Segment-Parallel Video Transcoding, Stitching & Audio Sync (`S6.P2.SP2`)

Parallelizing video transcoding across heterogeneous cluster nodes without introducing frame drops/duplications, codec extradata mismatches, or audio/video synchronization drift requires strict GOP-boundary partitioning, parameter normalization, and sample-accurate PTS/DTS re-basing.

#### 2.2.1 GOP Keyframe Scanner & Lossless Splitter (`GopSegmenter`)
- **Keyframe Boundary Partitioning**:
  - The source container is scanned at packet index level (`av_read_frame` packet headers) without decoding video frames to locate all IDR keyframes (`pkt.flags & AV_PKT_FLAG_KEY`), stream `time_base`, and exact frame counts per GOP.
  - Consecutive GOPs are clustered into `VideoSegmentDescriptor` units targeting $4.0..10.0$ seconds per segment adapted to cluster node capacity and total duration.
  - Invariant: Every video segment begins on an IDR keyframe, and the sum of `expected_frame_count` across all segments equals the source container frame count exactly.
- **Dedicated Continuous Audio Pipeline**:
  - Splitting and encoding lossy audio (AAC/Opus) per segment produces audible clicks, pops, and discontinuities at segment boundaries due to overlapping MDCT window boundary artifacts.
  - UMCF demuxes and encodes the audio stream as a continuous, unified pipeline alongside video segmentation, or passes uninterrupted audio through to the muxing stage.

#### 2.2.2 Heterogeneous Parallel Transcode Coordinator (`ParallelTranscodeCoordinator`)
- **Unified Encoding Profile Normalization**:
  - Across mixed hardware and software encoders (e.g., Node A using `h264_nvenc`, Node B using `h264_videotoolbox`, Node C using `h264_qsv`, and Node D using `libx264`), the coordinator enforces a uniform target profile:
    - Identical profile (`high` / `main10`), level, pixel format (`Nv12`, `P010le`), color primaries, matrix, and transfer characteristics.
    - Closed GOP (`-flags +cgop`) with fixed output timebase (`1/90000`).
    - Canonical 4-byte Annex-B start codes (`[0x00, 0x00, 0x00, 0x01]`) and deduplicated parameter sets via `ExtradataNormalizer`.
- **Capacity-Weighted Dispatch**:
  - Segments are dispatched proportionally to node capacity based on `benchmark_fps_1080p`, assigning larger segments to high-throughput discrete GPU encoders and lighter segments to mobile/laptop encoders.

#### 2.2.3 Sample-Accurate Stitcher & Fractional Framerate PTS Rebaser (`SampleAccurateStitcher`)
- **Exact Frame Parity Verification**:
  - Upon receiving completed segments from cluster workers, the stitcher verifies `actual_frame_count == expected_frame_count` and validates that each segment begins with an IDR keyframe.
- **Fractional Framerate PTS/DTS Precision**:
  - Fractional framerates ($23.976\text{ fps} = \frac{24000}{1001}$, $59.94\text{ fps} = \frac{60000}{1001}$) introduce cumulative floating-point rounding drift if timestamps are calculated naively.
  - `compute_fractional_pts` performs exact integer arithmetic in the stream timebase ($90{,}000\text{ Hz}$):
    $$\text{PTS}(n) = \left\lfloor \frac{n \times \text{timebase\_den} \times \text{fps\_den} + \frac{\text{fps\_num}}{2}}{\text{fps\_num}} \right\rfloor$$
  - Eliminates timestamp drift ($< 0.01\text{ ms}$ cumulative drift over $100{,}000$ frames).
- **Sub-Millisecond A/V Synchronization**:
  - Validates continuous audio sample alignment against re-based video timestamps at periodic intervals (e.g. flash/beep checkpoints at $t = 2\text{s}, 14\text{s}, 28\text{s}$), strictly enforcing maximum A/V drift $\le 1.0\text{ ms}$.
  - Monotonicity checks guarantee $DTS_i > DTS_{i-1}$ and $PTS_i > PTS_{i-1}$ with zero gaps or overlaps across segment seams.




