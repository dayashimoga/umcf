# Subphase S6.P2.SP2 Execution Prompt: Segment-Parallel Video Transcoding, Stitching & Audio Sync

## 1. Role & Context
You are the Distributed Video Pipeline & A/V Synchronization Engineer for UMCF. In Subphase `S6.P2.SP2`, you will implement `GopSegmenter`, `ParallelTranscodeCoordinator`, and `SampleAccurateStitcher` in `crates/umcf-media/src/segmenter.rs` and `src/stitcher.rs`. Splitting a video across heterogeneous devices without introducing frame duplication/drops, codec extradata (`SPS`/`PPS`/`VPS`) mismatches, or audio/video synchronization drift requires GOP-boundary keyframe splitting, unified codec parameter headers (`repeat-headers` / identical timebases), and sample-accurate PTS/DTS remuxing.

## 2. Prerequisites & Dependencies
- `S6.P2.SP1` (`HardwareCodecProber`, `MediaCodecAdapter`).
- `crates/umcf-scheduler` (heterogeneous node scheduling) and `crates/umcf-net` (QUIC chunk streaming).

## 3. Implementation Tasks
1. **GOP-Boundary Keyframe Scanner & Lossless Splitter (`crates/umcf-media/src/segmenter.rs`)**:
   - Scan the input container's packet index (`av_read_frame` / packet header index scan without decoding video frames) to locate all IDR/Keyframe positions (`pkt.flags & AV_PKT_FLAG_KEY`), exact PTS/DTS in stream `time_base`, and frame counts per GOP.
   - Group consecutive GOPs into `VideoSegmentDescriptor` units targeting `4.0..10.0` seconds per segment (adaptive to total duration and cluster node count):
     ```rust
     pub struct VideoSegmentDescriptor {
         pub segment_index: u32,
         pub start_pts: i64,
         pub end_pts: i64,
         pub time_base: (i32, i32),
         pub expected_frame_count: u64,
         pub byte_range_hint: Option<(u64, u64)>,
         pub source_blake3: [u8; 32],
     }
     ```
   - Demux each video segment losslessly (stream copy of compressed packets from keyframe $K_i$ to $K_{i+1}$) while extracting the audio stream(s) and subtitle tracks onto a dedicated continuous audio pipeline (preventing MDCT window boundary clicks/pops that occur if lossy AAC/Opus audio is split and encoded independently per segment).
2. **Heterogeneous Parallel Segment Dispatcher (`ParallelTranscodeCoordinator`)**:
   - Normalize encoding parameters across all participating nodes (even if Node A uses `h264_nvenc`, Node B uses `h264_videotoolbox`, and Node C uses `libx264`):
     - Enforce identical `profile` (`high`/`main10`), `level`, `pix_fmt`, `color_range`, `color_primaries`, `color_trc`, `colorspace`, fixed output `time_base` (e.g., `1/90000` or `1/60000`), closed GOP (`-flags +cgop`), and in-band parameter sets (`-bsf:v dump_extra` or unified Annex-B / AVCC extradata normalization).
   - Dispatch heavier/longer segments to desktop hardware encoders and shorter segments to mobile hardware encoders based on `CodecCapabilityDescriptor::benchmark_fps_1080p`.
3. **Sample-Accurate A/V Stitcher (`crates/umcf-media/src/stitcher.rs`)**:
   - Verify each returned transcoded segment has `actual_frame_count == expected_frame_count` and starts with an IDR keyframe.
   - Re-base segment PTS/DTS timestamps continuously so `Segment[i].first_pts == Segment[i-1].last_pts + frame_duration`, eliminating any rounding drift across variable or fractional framerates (`23.976`, `29.97`, `59.94` fps).
   - Mux the concatenated video stream with the continuously transcoded audio stream(s), subtitle tracks, and chapter metadata into the target container (`.mp4` with `faststart` moov atom, `.mkv`, or `.webm`).
4. **UI Integration**:
   - Flutter `TranscodeQueueScreen` & Svelte 5 `TranscodePipelineView.svelte`: Visual timeline bar divided into GOP segments color-coded by assigned node, showing live FPS per node, aggregate cluster FPS speedup (e.g., `412 fps (6.8x real-time)`), and stitching progress.

## 4. Acceptance Criteria
- >= 95% line/branch coverage in `segmenter.rs` and `stitcher.rs`; 100% test pass rate.
- **Exact Frame Parity**: A 1,800-frame source video split across 4 nodes using mixed encoders (`NVENC` + `VideoToolbox` + `libx264`) produces an output video with **exactly 1,800 decoded frames**.
- **Zero PTS/DTS Discontinuity & Zero A/V Drift**: Inspecting all packet timestamps of the stitched output verifies strictly monotonic DTS, zero timestamp gaps/overlaps at segment boundaries, and audio-to-video synchronization within <= 1.0 ms across the entire file.
- **Universal Decoder Playback**: Stitched output plays cleanly without decoder resets across hardware decoders, VLC, Chromium/Safari `<video>`, and `ffmpeg -v error -i out.mp4 -f null -`.

## 5. Required Tests
- **`[UNIT]`**:
  - `test_gop_keyframe_clustering_into_target_duration_segments`
  - `test_fractional_framerate_23_976_and_59_94_pts_rebaser_zero_drift`
  - `test_unified_sps_pps_extradata_normalizer_across_mixed_encoders`
- **`[INT]`**:
  - `test_4_node_mixed_encoder_transcode_and_stitch_exact_frame_count`: Transcode a 30-second test video (with a 1 kHz audio sync beep + visual flash every 2 seconds) across 4 workers, stitch into MP4, decode every frame and audio sample, and assert the visual flash and audio beep align within < 1 ms at $t = 2\text{s}, 14\text{s}, 28\text{s}$.
- **`[SIM]`**:
  - Variable Frame Rate (VFR) smartphone recording input transcoded to Constant Frame Rate (CFR) across 3 simulated nodes.
- **`[HW]`**:
  - Distributed 4K60 HEVC -> AV1/H.264 transcode across available hardware devices measuring end-to-end wall-clock speedup.
- **`[MANUAL]`**:
  - Play the stitched video in the Svelte 5 / Flutter built-in media player and scrub across segment boundaries verifying zero visual stutter or audio pop.

## 6. Required Evidence & Artifacts
- `artifacts/stage-06/p2/sp2_frame_parity_and_pts_audit.json`
- `artifacts/stage-06/p2/sp2_av_sync_flash_beep_alignment.json`

## 7. Documentation, `TODO.md` & `CHANGELOG.md` Updates
- Document GOP splitting, closed-GOP mixed-encoder normalization, and continuous audio muxing in `docs/architecture/GPU_RENDER_AND_MEDIA.md` (Section 2.2).
- Mark `S6.P2.SP2` complete in `TODO.md` and update `CHANGELOG.md`.

## 8. Rollback Criteria & Completion Gate
- **Rollback**: If stitched output drops or duplicates even a single frame, exhibits audio clicks at segment boundaries, or fails strict `ffmpeg -v error` decode validation.
- **Completion Gate**: All tests green with >= 95% coverage before proceeding to `S6.P2.SP3`.
