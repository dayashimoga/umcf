# Subphase S6.P2.SP1 Execution Prompt: Hardware-Accelerated Video/Audio Codec Probing & FFmpeg/Native Adapter

## 1. Role & Context
You are the Multimedia Codec & Hardware Acceleration Engineer for UMCF. In Subphase `S6.P2.SP1`, you will implement `HardwareCodecProber` and `MediaCodecAdapter` in `crates/umcf-media/src/prober.rs` and `src/adapter.rs`. Every node in a UMCF cluster (Android phone, iPhone/iPad, macOS Apple Silicon, Windows/Linux desktop with NVIDIA/AMD/Intel GPUs) has distinct hardware media engines with differing codec profiles, 10-bit HDR support, and concurrent session limits; your prober must empirically test and catalog these capabilities and expose a unified, safe Rust transcoding adapter.

## 2. Prerequisites & Dependencies
- `crates/umcf-hw` (GPU/SoC identification and thermal state).
- FFmpeg runtime/libraries or platform native media APIs (MediaCodec, VideoToolbox, NVENC, AMF, QSV, VAAPI).

## 3. Implementation Tasks
1. **Empirical Hardware Codec Prober (`crates/umcf-media/src/prober.rs`)**:
   - Define `CodecCapabilityDescriptor`:
     ```rust
     pub struct CodecCapabilityDescriptor {
         pub codec: MediaCodecId, // H264, Hevc, Av1, Vp9, ProRes, Aac, Opus, Flac
         pub direction: CodecDirection, // Encode, Decode
         pub backend: AccelerationBackend, // Nvenc, VideoToolbox, MediaCodec, Qsv, Amf, Vaapi, SoftwareCpu
         pub encoder_name: String, // e.g., "h264_nvenc", "hevc_videotoolbox", "c2.qti.avc.encoder"
         pub supported_pixel_formats: Vec<PixelFormat>, // Yuv420p, Nv12, P010le
         pub max_resolution: (u32, u32),
         pub supports_b_frames: bool,
         pub supports_10bit_hdr: bool,
         pub max_concurrent_sessions: u8,
         pub benchmark_fps_1080p: f32,
     }
     ```
   - Implement `HardwareCodecProber::probe_all()`:
     - Queries available encoders/decoders and runs a fast 5-frame $640 \times 360$ synthetic test encode in memory (< 40 ms per candidate hardware encoder) to verify that the hardware driver is actually functional (catching headless Linux boxes with `h264_nvenc` listed but no NVIDIA driver loaded, or Android devices where secondary hardware codecs fail initialization).
     - Caches probe results keyed by `(os_version, driver_version, binary_hash)`.
2. **Unified Safe Media Codec Adapter (`crates/umcf-media/src/adapter.rs`)**:
   - Implement `MediaCodecAdapter` executing isolated transcoding jobs with:
     - **Strict Argument & Path Sandboxing**: Never construct shell strings; pass structured argument arrays or library API structs with read-only input file descriptors and isolated scratch output directories.
     - **Real-Time Progress Telemetry Parser**: Stream frame index, current FPS, bit_rate, q-factor, dup/drop frame counters, and `out_time_us` at 10 Hz over an async channel.
     - **Hardware-to-Software Automatic Fallback**: If a hardware encoder fails initialization at runtime (e.g., `NVENC_ERR_OUT_OF_MEMORY` or concurrent session limit reached), automatically translate rate-control flags (`-rc vbr -cq` -> `-crf -preset`) to the deterministic software encoder (`libx264`, `libx265`, `libsvtav1`) so the segment succeeds without uniform parameter mismatch.
     - **Clean Process/Context Teardown**: Ensure killing/cancelling a `MediaCodecAdapter` task immediately terminates child media contexts/processes and cleans up temporary buffers within <= 50 ms.
3. **UI Integration**:
   - Flutter & Svelte 5: Hardware Media Engines card in Device Details showing verified hardware encoders/decoders (H.264, HEVC, AV1), 10-bit HDR support badge, and 1080p benchmark FPS.

## 4. Acceptance Criteria
- >= 95% line/branch coverage in `prober.rs` and `adapter.rs`; 100% test pass rate.
- `HardwareCodecProber` accurately distinguishes between compiled-in codec names and actually usable hardware encoders via the 5-frame synthetic verification test.
- When hardware session exhaustion is simulated, `MediaCodecAdapter` transparently falls back to software encoding with matching resolution, pixel format, codec profile, and target bitrate/CRF.
- Cancelling an active transcode task leaves zero zombie processes or leaked temporary files.

## 5. Required Tests
- **`[UNIT]`**:
  - `test_codec_capability_discovery_and_ghost_driver_filtering`
  - `test_hw_to_sw_encoder_parameter_translation_h264_hevc_av1`
  - `test_realtime_telemetry_progress_line_parser`
- **`[INT]`**:
  - `test_encode_decode_synthetic_colorbars_h264_and_opus`
  - `test_hardware_session_limit_exhaustion_triggers_software_fallback`
  - `test_mid_transcode_cancellation_cleans_up_resources_under_50ms`
- **`[SIM]`**:
  - Simulate driver hang during hardware encode initialization and verify timeout + fallback to CPU encoder.
- **`[HW]`**:
  - Run `HardwareCodecProber::probe_all()` on host machine and verify hardware encoder detection and 1080p synthetic encode FPS.
- **`[MANUAL]`**:
  - View the "Hardware Media Capabilities" matrix in Flutter and Svelte 5 Device Inspector.

## 6. Required Evidence & Artifacts
- `artifacts/stage-06/p2/sp1_hardware_codec_probe_report.json`
- `artifacts/stage-06/p2/sp1_fallback_and_cancellation_test.json`

## 7. Documentation, `TODO.md` & `CHANGELOG.md` Updates
- Document hardware codec probing and parameter normalization rules in `docs/architecture/GPU_RENDER_AND_MEDIA.md` (Section 2.1).
- Mark `S6.P2.SP1` complete in `TODO.md` and update `CHANGELOG.md`.

## 8. Rollback Criteria & Completion Gate
- **Rollback**: Any command-injection vector in media arguments, zombie transcode processes after cancellation, or mismatched codec profile/pixel format when falling back from HW to SW.
- **Completion Gate**: All tests green with >= 95% coverage before proceeding to `S6.P2.SP2`.
