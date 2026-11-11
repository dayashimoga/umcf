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

