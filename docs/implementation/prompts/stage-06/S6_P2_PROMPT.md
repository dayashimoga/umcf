# Phase S6.P2 Execution Prompt: Distributed Media Transcoding & Processing Pipeline

## 1. Role & Context
You are the Multimedia Systems & Distributed Video Pipeline Lead for UMCF. Phase `S6.P2` builds `crates/umcf-media`, enabling the cluster to pool hardware video encoders/decoders (NVIDIA NVENC, Apple VideoToolbox, Android MediaCodec, Intel QSV, AMD AMF, Linux VAAPI, and CPU `libx264`/`libx265`/`libsvtav1` fallbacks) to transcode, scale, filter, and process video and audio workloads in parallel across multiple devices with sample-accurate audio synchronization and automated visual quality verification.

This phase consists of three sequential subphases:
1. `S6_P2_SP1.md`: Hardware-Accelerated Video/Audio Codec Probing & FFmpeg/Native Adapter
2. `S6_P2_SP2.md`: Segment-Parallel Video Transcoding, Stitching & Audio Sync
3. `S6_P2_SP3.md`: Media Pipeline Fault Recovery & Quality Verification

## 2. Prerequisites & Dependencies
- Completion of Phase `S6.P1` and Stage 1–5 foundations (`umcf-scheduler`, `umcf-net`, `umcf-hw`).
- FFmpeg libraries/CLI or native platform media bridges configured and verified by `crates/umcf-media`.

## 3. Implementation Tasks
1. **Subphase S6.P2.SP1 (`crates/umcf-media/src/prober.rs`, `src/adapter.rs`)**:
   - Probe all hardware and software video/audio codecs (H.264/AVC, H.265/HEVC, AV1, VP9, ProRes, AAC, Opus, FLAC), supported pixel formats (`NV12`, `YUV420P`, `P010LE`), max resolution/FPS, B-frame support, and concurrent hardware encoder session limits.
   - Implement `MediaCodecAdapter` wrapping platform hardware accelerators and deterministic software fallbacks with unified progress telemetry (encoded FPS, bitrate, speed factor, frame drops).
2. **Subphase S6.P2.SP2 (`crates/umcf-media/src/segmenter.rs`, `src/stitcher.rs`)**:
   - Implement lossless GOP (Group of Pictures) keyframe demuxer/splitter (`split_at_idr_keyframes`) that partitions source media into independent scene/GOP segments without re-encoding.
   - Dispatch video segments in parallel across cluster nodes while transcoding/normalizing audio on a single high-precision pipeline (or frame-aligned FLAC/Opus chunks) and stitching completed segments into a seamless MP4/MKV/WebM container with monotonic PTS/DTS timestamps and zero A/V sync drift.
3. **Subphase S6.P2.SP3 (`crates/umcf-media/src/quality_auditor.rs`, `src/fault_recovery.rs`)**:
   - Verify every transcoded segment's frame count, exact PTS duration, codec profile/level compliance, and sampled SSIM/PSNR/VMAF score; automatically re-transcode any segment that fails quality thresholds or stalls on a throttled mobile device.

## 4. Acceptance Criteria
- >= 95% line/branch coverage across `crates/umcf-media`; 100% test pass rate.
- Stitched multi-node output video has **exact frame count parity** with the source video (`decoded_frames_out == decoded_frames_in`) and **zero PTS/DTS non-monotonicity or A/V sync drift** (< 1 ms audio/video alignment error across segment boundaries).
- Hardware encoder failure (e.g., exceeding NVENC/MediaCodec concurrent session limit) automatically falls back to software encoding or another cluster node without failing the job.
- Sampled SSIM verification rejects corrupted/macroblocked segments (SSIM < 0.92) and re-queues them automatically.

## 5. Required Tests
- **`[UNIT]`**: Codec capability parser, IDR keyframe index analyzer, PTS/DTS continuity normalizer, MP4/MKV concat manifest builder, SSIM quality threshold evaluator.
- **`[INT]`**: 4-worker distributed transcode of a 30-second multi-GOP video + stereo audio track into H.264/HEVC/AV1 MP4, followed by full `ffprobe`/demux frame-by-frame PTS audit.
- **`[SIM]`**: Worker #2 crashes mid-segment and Worker #3 returns a truncated segment missing 5 frames; verify coordinator detects frame count mismatch and recovers.
- **`[HW]`**: Hardware-accelerated encode/decode benchmark on NVENC, VideoToolbox, VAAPI, or Android MediaCodec.
- **`[MANUAL]`**: Submit a video transcode job in Flutter `TranscodeQueueScreen` & Svelte 5 `/media`, watch per-segment node assignment timeline, and play back the stitched video verifying smooth segment transitions.

## 6. Required Evidence & Artifacts
- `artifacts/stage-06/p2/codec_probe_matrix.json`
- `artifacts/stage-06/p2/stitched_pts_dts_continuity_audit.json`
- `artifacts/stage-06/p2/segment_ssim_quality_report.json`

## 7. Documentation, `TODO.md` & `CHANGELOG.md` Updates
- Document GOP splitting, PTS/DTS normalization, and hardware encoder fallback in `docs/architecture/GPU_RENDER_AND_MEDIA.md` (Part 2).
- Mark `S6.P2.SP1`–`S6.P2.SP3` complete in `TODO.md` and update `CHANGELOG.md`.

## 8. Rollback Criteria & Completion Gate
- **Rollback**: Any audio pop/click or video freeze/glitch at stitched segment boundaries, or orphaned temporary segment files left on worker disks after job completion.
- **Gate**: All 3 subphases (`S6_P2_SP1`..`SP3`) pass with >= 95% coverage before starting Phase `S6.P3`.
