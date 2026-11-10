# Subphase S6.P3.SP1 Execution Prompt: Display Capture & Hardware-Accelerated H.264/HEVC/AV1 Encoding Host

## 1. Role & Context
You are the Low-Latency Graphics Capture & Hardware Video Encoding Engineer for UMCF. In Subphase `S6.P3.SP1`, you will implement `DisplayCaptureEngine`, `LoopbackAudioCapturer`, and `ZeroLatencyVideoEncoder` in `crates/umcf-stream/src/capture.rs` and `src/low_latency_encoder.rs`. Achieving console-grade remote streaming requires capturing the display surface directly in GPU VRAM (avoiding CPU system RAM copies whenever possible), converting RGB to NV12/P010 in a sub-millisecond compute shader, and feeding the hardware encoder in ultra-low-latency zero-B-frame mode.

## 2. Prerequisites & Dependencies
- `S6.P1.SP1` (`umcf-gpu` compute shader color conversion & texture sharing).
- `S6.P2.SP1` (`umcf-media` hardware codec prober).

## 3. Implementation Tasks
1. **Cross-Platform Display & Window Capture Engine (`crates/umcf-stream/src/capture.rs`)**:
   - Define `CaptureTarget`: `PrimaryMonitor`, `MonitorById(u32)`, `ApplicationWindow(u64)`, or `VirtualHeadlessDisplay { width, height, refresh_rate_hz }`.
   - Implement platform backends behind the `DisplayCapturer` trait:
     - **Windows**: DXGI Desktop Duplication API (`IDXGIOutputDuplication`) & `Windows.Graphics.Capture` providing D3D11/D3D12 shared GPU textures, hardware cursor shape/position metadata extraction, and dirty-rectangle metadata.
     - **Linux**: PipeWire (`xdg-desktop-portal` ScreenCast with DMA-BUF file descriptors) + X11 `XShmGetImage` / `XComposite` fallback.
     - **macOS**: `ScreenCaptureKit` (`SCStream`) delivering `CMSampleBuffer` / `IOSurface` GPU-backed buffers.
     - **Headless / Test Fallback**: Synthetic high-FPS GPU render target generator with embedded binary frame-counter barcode and sub-millisecond timestamp clock for automated CI latency measurement.
   - Implement GPU compute shader RGB/BGRA -> NV12 (8-bit BT.709) and P010 (10-bit HDR BT.2020 PQ) chroma subsampling (`crates/umcf-stream/src/shaders/rgb_to_nv12.wgsl`) executing in < 0.5 ms on the GPU.
2. **Ultra-Low-Latency Hardware Video Encoder (`crates/umcf-stream/src/low_latency_encoder.rs`)**:
   - Configure hardware encoders (NVENC, AMF, QSV, VideoToolbox, VAAPI) and `libx264` (`-tune zerolatency -preset ultrafast`) fallback with strict real-time streaming invariants:
     - **Zero Lookahead & Zero B-Frames**: `max_b_frames = 0`, `rc_lookahead = 0`, `delay = 0`.
     - **Single-Frame VBV Buffer**: `vbv_bufsize = bitrate / fps` (with 1.2x burst tolerance) so no single frame spikes network queuing delay.
     - **Periodic Intra-Refresh & On-Demand IDR**: Support periodic GDR (Gradual Decoder Refresh) wave columns + instant `force_idr_on_next_frame()` atomic trigger when the transport layer reports an unrecoverable packet loss (PLI).
   - Attach `EncodedFrameMetadata { frame_seq, capture_timestamp_us, encode_start_us, encode_end_us, is_idr, cursor_pos, cursor_shape_hash }` to every emitted access unit.
3. **Low-Latency System Audio Loopback (`LoopbackAudioCapturer`)**:
   - Capture host desktop/application audio at 48 kHz stereo and encode into 5 ms or 10 ms Opus packets (`OPUS_APPLICATION_RESTRICTED_LOWDELAY`) with in-band FEC enabled.
4. **UI Integration**:
   - Flutter & Svelte 5 Host Settings: Monitor/Window selector with live thumbnail preview, codec selector (`Auto`, `AV1`, `HEVC (10-bit HDR)`, `H.264`), target FPS (`30`, `60`, `90`, `120`), and privacy "Hide Sensitive Notifications / Blank Host Screen" toggle.

## 4. Acceptance Criteria
- >= 95% line/branch coverage in `capture.rs` and `low_latency_encoder.rs`; 100% test pass rate.
- Median `capture + color_convert + encode` latency is <= 6.0 ms at 1080p60 on hardware encoders (and <= 10.0 ms on software `ultrafast` fallback).
- Bitstream analyzer confirms `num_reorder_frames == 0` (zero B-frames) and immediate single-frame output on every `encode_frame()` call (zero frame buffering delay).
- Calling `force_idr_on_next_frame()` guarantees the very next emitted frame is an instantaneous IDR keyframe with in-band `SPS`/`PPS`/`VPS` headers.

## 5. Required Tests
- **`[UNIT]`**:
  - `test_rgb_to_nv12_and_p010_bt709_shader_color_accuracy`
  - `test_single_frame_vbv_buffer_sizing_and_zero_bframe_config`
  - `test_on_demand_idr_atomic_trigger_emits_keyframe_immediately`
  - `test_opus_lowdelay_5ms_audio_packetizer`
- **`[INT]`**:
  - `test_capture_to_encode_pipeline_60fps_sustained_zero_frame_lag`: Run 300 frames at 60 FPS through `DisplayCapturer` + `ZeroLatencyVideoEncoder`, verifying every input frame $N$ produces output packet $N$ before frame $N+1$ is submitted, with median encode time < 6 ms.
- **`[SIM]`**:
  - Rapid dynamic resolution & bitrate reconfiguration (e.g., 1080p @ 20 Mbps -> 720p @ 4 Mbps) without recreating the OS capture session.
- **`[HW]`**:
  - Benchmark real desktop display capture + hardware H.264/HEVC/AV1 encoding over 1,000 frames on target OS.
- **`[MANUAL]`**:
  - Open Host Stream Settings in Flutter/Svelte UI, select a specific monitor or application window, and verify the live preview and capture FPS counter.

## 6. Required Evidence & Artifacts
- `artifacts/stage-06/p3/sp1_capture_encode_latency_percentiles.json`
- `artifacts/stage-06/p3/sp1_bitstream_zero_bframe_verification.json`

## 7. Documentation, `TODO.md` & `CHANGELOG.md` Updates
- Document zero-copy GPU capture, NV12 compute conversion, and zero-B-frame encoder tuning in `docs/architecture/LOW_LATENCY_STREAMING.md` (Section 1).
- Mark `S6.P3.SP1` complete in `TODO.md` and update `CHANGELOG.md`.

## 8. Rollback Criteria & Completion Gate
- **Rollback**: If the encoder buffers >= 1 frame of lookahead delay, if GPU textures leak during capture resolution changes, or if `force_idr` fails to include parameter sets (`SPS`/`PPS`).
- **Completion Gate**: All tests green with >= 95% coverage before proceeding to `S6.P3.SP2`.
