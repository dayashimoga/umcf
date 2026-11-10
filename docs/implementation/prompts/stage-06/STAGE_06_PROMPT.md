# Stage 06 Execution Prompt: GPU Compute, Rendering, Media Processing & Remote Application/Game Streaming

## 1. Role & Context
You are the Principal GPU Compute, Media Pipeline & Real-Time Streaming Architect for the **Unified Mobile-First Compute Fabric (UMCF)**. Stage 6 implements cross-platform GPU compute shader dispatch and distributed tile/frame rendering (`crates/umcf-gpu`), hardware-accelerated segment-parallel media transcoding (`crates/umcf-media`), and ultra-low-latency (< 25 ms LAN glass-to-glass) remote desktop/application/game streaming with multi-modal input return channels (`crates/umcf-stream`).

Every subphase in Stage 6 follows the strict UMCF hierarchy:
`Stage → Phase → Subphase → Implementation tasks → Acceptance criteria → Tests → Evidence → Documentation`.

## 2. Stage 06 Phase & Subphase Index

| Phase | Subphase | Prompt File | Title & Scope |
| :--- | :--- | :--- | :--- |
| **S6.P1** | — | `docs/implementation/prompts/stage-06/S6_P1_PROMPT.md` | **Cross-Platform GPU Compute & Distributed Rendering Engine** |
| S6.P1 | S6.P1.SP1 | `docs/implementation/prompts/stage-06/S6_P1_SP1.md` | WGPU/Vulkan/Metal Compute Shader Dispatch & Buffer Validation |
| S6.P1 | S6.P1.SP2 | `docs/implementation/prompts/stage-06/S6_P1_SP2.md` | Tile/Frame-Partitioned Distributed Rendering Coordinator & Compositor |
| S6.P1 | S6.P1.SP3 | `docs/implementation/prompts/stage-06/S6_P1_SP3.md` | Deterministic Verification & Checksum Auditing of Rendered Outputs |
| **S6.P2** | — | `docs/implementation/prompts/stage-06/S6_P2_PROMPT.md` | **Distributed Media Transcoding & Processing Pipeline** |
| S6.P2 | S6.P2.SP1 | `docs/implementation/prompts/stage-06/S6_P2_SP1.md` | Hardware-Accelerated Video/Audio Codec Probing & FFmpeg/Native Adapter |
| S6.P2 | S6.P2.SP2 | `docs/implementation/prompts/stage-06/S6_P2_SP2.md` | Segment-Parallel Video Transcoding, Stitching & Audio Sync |
| S6.P2 | S6.P2.SP3 | `docs/implementation/prompts/stage-06/S6_P2_SP3.md` | Media Pipeline Fault Recovery & Quality Verification |
| **S6.P3** | — | `docs/implementation/prompts/stage-06/S6_P3_PROMPT.md` | **Low-Latency Remote Desktop & Game Streaming Host/Client** |
| S6.P3 | S6.P3.SP1 | `docs/implementation/prompts/stage-06/S6_P3_SP1.md` | Display Capture & Hardware-Accelerated H.264/HEVC/AV1 Encoding Host |
| S6.P3 | S6.P3.SP2 | `docs/implementation/prompts/stage-06/S6_P3_SP2.md` | Ultra-Low-Latency QUIC/WebRTC Media Transport & Adaptive Bitrate |
| S6.P3 | S6.P3.SP3 | `docs/implementation/prompts/stage-06/S6_P3_SP3.md` | Multi-Modal Input Return Channel (Touch, Gamepad, Keyboard, Mouse, Stylus) |

## 3. Prerequisites & Dependencies
- **Stage 05 Completion Gate**: All Stage 1–5 crates and tests passing at >= 95% coverage.
- **Graphics & Media Dependencies**: `wgpu` (Vulkan, Metal, DX12, WebGPU backends), `naga` (WGSL/SPIR-V validator), FFmpeg >= 6.1 libraries/CLI or platform hardware media APIs (MediaCodec on Android, VideoToolbox on macOS/iOS, NVENC/AMF/QSV/VAAPI on Windows/Linux), and platform display/input APIs (DXGI Desktop Duplication / Windows.Graphics.Capture, PipeWire/Wayland/X11 shm, ScreenCaptureKit).

## 4. Implementation Tasks (Stage-Level)
1. **Workspace Crates**:
   - `crates/umcf-gpu`: Sandboxed WGSL/SPIR-V compute shader validator, bounded VRAM buffer allocator, GPU watchdog timer, dynamic tile/frame rendering coordinator, and perceptual/checksum output verifier.
   - `crates/umcf-media`: Hardware encoder/decoder capability prober, GOP-aligned video splitter, parallel segment dispatcher, lossless fMP4/MKV stitcher with sample-accurate audio sync, and VMAF/SSIM/PSNR quality auditor.
   - `crates/umcf-stream`: Zero-copy screen & audio capture host, low-latency hardware video encoder, GCC/TWCC-inspired QUIC/WebRTC media transport with FEC, and low-jitter hid/input injection return channel.
2. **Protocol & API Endpoints**:
   - `proto/umcf/gpu/v1/render.proto`, `proto/umcf/media/v1/transcode.proto`, `proto/umcf/stream/v1/remote_stream.proto`.
3. **UI Screens**:
   - Flutter (`ui/flutter_app/lib/features/`): `render/DistributedRenderStudioScreen.dart`, `media/TranscodeQueueScreen.dart`, `stream/RemoteStreamClientView.dart` (with customizable virtual gamepad/touch overlay & HUD).
   - Svelte 5 (`ui/admin_web/src/routes/`): `gpu/+page.svelte` (Tile Render Grid & Shader Sandbox), `media/+page.svelte` (Segment Transcode Pipeline), `stream/+page.svelte` (WebCodecs/WebRTC Browser Stream Client & Telemetry).

## 5. Acceptance Criteria
- >= 95.0% line and branch coverage across `crates/umcf-gpu`, `crates/umcf-media`, and `crates/umcf-stream` with 100% test pass rate.
- Untrusted WGSL/SPIR-V shaders with out-of-bounds array accesses or infinite loops are rejected or preempted without crashing the host GPU driver (TDR protection).
- Distributed tile rendering across 4 heterogeneous GPU backends produces a composited frame with SSIM >= 0.9999 (and exact checksum on integer compute buffers) compared to single-node rendering.
- Distributed video transcoding of a 60-second 4K60 test video across 4 nodes completes with zero A/V sync drift (< 1 ms PTS discontinuity) and bit-seamless GOP boundaries.
- Remote streaming achieves <= 25 ms median end-to-end glass-to-glass latency over 1 Gbps LAN / Wi-Fi 6 at 1080p60 and <= 3 ms input return channel latency.

## 6. Required Tests
- **Unit (`[UNIT]`)**: Naga WGSL bounds injector, tile work-stealing splitter, GOP keyframe parser, PTS/DTS audio resampler math, FEC packet recovery, virtual gamepad/touch coordinate normalization.
- **Integration (`[INT]`)**: Multi-node GPU compute buffer map/reduce, 4-node video transcode + stitch + SSIM verification, loopback screen capture -> encode -> QUIC transport -> decode -> frame compare.
- **Simulation (`[SIM]`)**: Byzantine worker returning corrupted render tile or transcode segment; 5% UDP packet loss and bandwidth drop from 50 Mbps to 8 Mbps during live game stream.
- **Hardware-Required (`[HW]`)**: Vulkan/Metal/DX12 shader dispatch, NVENC/VideoToolbox/MediaCodec hardware transcode, real Android touch/gamepad streaming to desktop host.
- **Manually Verified (`[MANUAL]`)**: Live tile assembly canvas in Svelte/Flutter, side-by-side transcode preview, and playing a 60 FPS interactive 3D scene over the Flutter remote streaming client.

## 7. Required Evidence & Artifacts
- `artifacts/stage-06/coverage/lcov.info` (>= 95% coverage).
- `artifacts/stage-06/gpu/tile_compositor_ssim_audit.json`.
- `artifacts/stage-06/media/transcode_av_sync_and_vmaf_report.json`.
- `artifacts/stage-06/stream/glass_to_glass_latency_histogram.json`.

## 8. Documentation, `TODO.md` & `CHANGELOG.md` Updates
- Create `docs/architecture/GPU_RENDER_AND_MEDIA.md` and `docs/architecture/LOW_LATENCY_STREAMING.md`.
- Mark all `S6.P1.*`, `S6.P2.*`, and `S6.P3.*` tasks `[x]` in `TODO.md` and update `CHANGELOG.md`.

## 9. Rollback Criteria & Completion Gate
- **Rollback Criteria**: Any GPU TDR hang caused by unvalidated shader loops, A/V desync > 15 ms in stitched media, or unauthenticated input injection in `umcf-stream`.
- **Completion Gate**: Stage 07 (`stage-07`) may not begin until `S6.P1`, `S6.P2`, and `S6.P3` pass all completion gates.
