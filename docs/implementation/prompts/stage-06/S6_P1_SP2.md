# Subphase S6.P1.SP2 Execution Prompt: Tile/Frame-Partitioned Distributed Rendering Coordinator & Compositor

## 1. Role & Context
You are the Distributed Rendering & Parallel Graphics Engineer for UMCF. In Subphase `S6.P1.SP2`, you will implement `DistributedRenderCoordinator` and `FrameCompositor` in `crates/umcf-gpu/src/render_coordinator.rs` and `src/compositor.rs`. This engine partitions high-resolution 2D still renders (e.g., 4K/8K path-tracing or fractals) into spatial tiles ordered along a space-filling Hilbert/Morton curve and multi-frame 3D animations into frame/tile batches, dispatching them across heterogeneous mobile and desktop GPUs with dynamic work-stealing and real-time progressive compositing.

## 2. Prerequisites & Dependencies
- `S6.P1.SP1` (`GpuComputeDispatcher`, `ValidatedShader`).
- `S5.P1.SP1` (`ContentAddressedModelStore` / asset store for shared scene geometry & textures).
- `crates/umcf-scheduler` (heterogeneous node capability scoring & work-stealing queues).

## 3. Implementation Tasks
1. **Adaptive Tile & Frame Decomposition (`crates/umcf-gpu/src/render_coordinator.rs`)**:
   - Define `RenderJobSpec`:
     ```rust
     pub struct RenderJobSpec {
         pub job_id: JobId,
         pub resolution: (u32, u32), // e.g., (3840, 2160)
         pub frame_range: RangeInclusive<u32>,
         pub samples_per_pixel: u32,
         pub tile_size: (u32, u32),  // default (128, 128), adaptive 64..512
         pub pixel_format: RenderPixelFormat, // Rgba8UnormSrgb, Rgba16Float, Rgba32Float
         pub scene_asset_root: Blake3Hash,
         pub shader_module: ValidatedShader,
     }
     ```
   - Generate `TileDescriptor { frame_idx, tile_id, x, y, width, height, morton_index }` sorted in spiral-from-center or Hilbert-curve order so focal center regions render first for immediate user feedback.
   - **Adaptive Tile Sizing & Work-Stealing Queue**:
     - Assign larger tile batches (e.g., 4x `256x256` tiles) to high-TFLOPS desktop GPUs and single `64x64` or `128x128` tiles to mobile GPUs.
     - Maintain an EWMA throughput tracker (`pixels_per_ms`) per worker node.
     - **Tail-Latency Speculative Execution (Straggler Mitigation)**: When >= 95% of tiles in a frame are complete and a slow mobile node is still processing a remaining tile past `1.5x` the fastest node's expected completion time, speculatively dispatch that tile to an idle fast node and accept whichever finishes first.
2. **Progressive Frame Compositor (`crates/umcf-gpu/src/compositor.rs`)**:
   - Implement `FrameCompositor` maintaining an in-memory/mmap frame buffer (`Rgba8` / `Rgba16Float` / OpenEXR).
   - As each `CompletedTilePacket` arrives over QUIC:
     - Decompress tile payload (LZ4 / Zstd for float buffers, lossless WebP/PNG for 8-bit buffers).
     - Blit tile into the master frame buffer at `(x, y)` with optional overlap borderfeathering / alpha blending.
     - Emit a `ProgressiveFrameDelta` event over WebSocket/gRPC to connected UI clients so the user sees each tile pop into place with a colored border indicating which device rendered it.
   - Export completed frames as lossless PNG, 16/32-bit OpenEXR (`.exr`), or raw float buffers.
3. **UI Integration**:
   - Flutter `DistributedRenderStudioScreen` & Svelte 5 `RenderCanvas.svelte`: Live progressive canvas with toggleable "Node Attribution Overlay" (color-coding each rendered tile by device name, e.g., Blue = Desktop RTX, Green = Pixel 8 Pro, Purple = MacBook M3) and per-device tile completion counters.

## 4. Acceptance Criteria
- >= 95% line/branch coverage in `render_coordinator.rs` and `compositor.rs`; 100% test pass rate.
- Rendering a $1920 \times 1080$ frame partitioned into $128 \times 128$ tiles (135 tiles) across 4 simulated heterogeneous workers (relative speeds `10x`, `5x`, `2x`, `1x`) completes with zero missing or overlapping pixel seams and achieves >= 92% of ideal linear speedup.
- Speculative tail execution eliminates straggler delays when a worker stalls on the final 5% of tiles.
- OpenEXR (`.exr`) and PNG exports preserve exact pixel values from the composited buffer.

## 5. Required Tests
- **`[UNIT]`**:
  - `test_hilbert_and_center_spiral_tile_generation_non_multiple_resolutions`
  - `test_adaptive_tile_batch_sizing_by_worker_throughput_ewma`
  - `test_speculative_tail_dispatch_cancels_redundant_slow_worker`
  - `test_compositor_blit_exact_pixel_placement_and_exr_export`
- **`[INT]`**:
  - `test_4_node_distributed_path_tracer_progressive_composition`: Dispatch a procedural 3D Cornell Box WGSL path-tracing shader across 4 workers, stream progressive tile deltas, and verify the final composited `1024x1024` image.
- **`[SIM]`**:
  - Inject a 10-second stall on worker #3 on tile #130 of 135; verify speculative re-dispatch to worker #1 finishes the frame within < 100 ms.
- **`[HW]`**:
  - Run a distributed 4K fractal/raymarching render across desktop + mobile GPUs and measure wall-clock speedup vs single device.
- **`[MANUAL]`**:
  - Toggle the "Node Attribution Overlay" in Flutter & Svelte 5 during a live render and inspect the tile attribution tooltip (device name, render time in ms, GPU backend).

## 6. Required Evidence & Artifacts
- `artifacts/stage-06/p1/sp2_heterogeneous_speedup_efficiency.json`
- `artifacts/stage-06/p1/sp2_composited_cornell_box_checksum.json`

## 7. Documentation, `TODO.md` & `CHANGELOG.md` Updates
- Document spatial tile decomposition, adaptive batching, and speculative tail execution in `docs/architecture/GPU_RENDER_AND_MEDIA.md` (Section 1.2).
- Mark `S6.P1.SP2` complete in `TODO.md` and update `CHANGELOG.md`.

## 8. Rollback Criteria & Completion Gate
- **Rollback**: Any pixel seam/artifact at tile boundaries, memory leak on multi-frame sequences, or straggler deadlock when a worker disconnects.
- **Completion Gate**: All tests green with >= 95% coverage before proceeding to `S6.P1.SP3`.
