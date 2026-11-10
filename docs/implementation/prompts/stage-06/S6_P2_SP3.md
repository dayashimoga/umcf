# Subphase S6.P2.SP3 Execution Prompt: Media Pipeline Fault Recovery & Quality Verification

## 1. Role & Context
You are the Media Quality Assurance & Resilient Pipeline Engineer for UMCF. In Subphase `S6.P2.SP3`, you will implement `MediaQualityAuditor` and `SegmentFaultRecoveryManager` in `crates/umcf-media/src/quality_auditor.rs` and `src/fault_recovery.rs`. Faulty mobile hardware encoders, bit-rot in transit, or misconfigured rate-control on a remote node can produce corrupted bitstreams, dropped frames, or severe macroblocking; UMCF must automatically audit every transcoded segment before stitching and transparently re-dispatch failed or stalled segments.

## 2. Prerequisites & Dependencies
- `S6.P2.SP1` (`MediaCodecAdapter`) and `S6.P2.SP2` (`GopSegmenter`, `ParallelTranscodeCoordinator`, `SampleAccurateStitcher`).

## 3. Implementation Tasks
1. **Automated Segment Structural & Perceptual Quality Auditor (`crates/umcf-media/src/quality_auditor.rs`)**:
   - Implement `MediaQualityAuditor::verify_segment(&SourceSegment, &TranscodedSegment, &QualityPolicy) -> Result<SegmentQualityReport, QualityError>`:
     - **Structural Bitstream Check**:
       1. Verify container/bitstream demuxes with 0 decoder errors.
       2. Verify `decoded_frame_count == source_segment.expected_frame_count`.
       3. Verify first frame is an IDR keyframe and resolution/pixel format/codec profile match `TranscodeJobSpec`.
       4. Verify segment file size / average bitrate is within sane bounds (rejecting 0-byte or all-black encoder hardware glitches).
     - **Sampled Perceptual Quality Check (SSIM / PSNR / VMAF-Proxy)**:
       - Decode 3 anchor frames per segment (first keyframe, middle frame, last frame) on both source and transcoded segment (scaled to matching resolution) and compute Y-channel SSIM and PSNR.
       - Reject segment if `min_anchor_ssim < quality_policy.min_ssim_threshold` (default `0.92` for standard bitrates) or `min_anchor_psnr < 30.0 dB`.
2. **Segment Fault Recovery & Straggler Preemption (`crates/umcf-media/src/fault_recovery.rs`)**:
   - Implement `SegmentFaultRecoveryManager`:
     - **Worker Disconnect / Timeout Recovery**: Track heartbeat progress (`encoded_frames / expected_frames`) every 1 second. If a worker disconnects or makes zero frame progress for `> 5.0 seconds`, mark the attempt aborted and immediately re-queue the segment on the highest-throughput healthy node.
     - **Hardware Encoder Quirk Fallback**: If a segment fails `MediaQualityAuditor` on a node's hardware encoder (e.g., buggy mobile hardware encoder producing corrupted blocks on a specific scene), blacklist that node's hardware encoder for that segment and re-run using either a different node or deterministic software encoding (`libx264`/`libx265`/`libsvtav1`).
     - **Checkpointed Job Resume**: Persist completed, verified segments in the coordinator's scratch store so even if the coordinator daemon restarts, a long movie transcode resumes from only the unfinished segments.
3. **UI Integration**:
   - Flutter & Svelte 5: Quality telemetry badge per completed job (`Avg SSIM: 0.978 | Min PSNR: 41.2 dB | Frame Parity: 100% | Retried Segments: 1`) and expandable segment health grid.

## 4. Acceptance Criteria
- >= 95% line/branch coverage in `quality_auditor.rs` and `fault_recovery.rs`; 100% test pass rate.
- 100% of truncated segments (missing >= 1 frame), bitstream-corrupted segments, or severely degraded segments (SSIM < threshold) are caught by `MediaQualityAuditor` before `SampleAccurateStitcher` runs.
- Killing 2 out of 4 worker nodes mid-transcode results in automatic segment reassignment and a 100% valid stitched output file with zero user intervention.
- Restarting the coordinator mid-job resumes from already-verified segments on disk without re-transcoding completed segments.

## 5. Required Tests
- **`[UNIT]`**:
  - `test_structural_frame_count_and_idr_keyframe_validator`
  - `test_3_anchor_frame_y_channel_ssim_and_psnr_evaluator`
  - `test_stalled_worker_progress_watchdog_and_segment_requeue`
- **`[INT]`**:
  - `test_corrupted_segment_and_truncated_segment_caught_and_retranscoded_on_fallback_node`
  - `test_coordinator_crash_recovery_resumes_from_verified_segment_checkpoints`
- **`[SIM]`**:
  - Chaos transcode of a 20-segment video across 5 simulated workers with a 25% random worker kill rate and 10% corrupted-output injection rate, verifying 100% eventual job completion and frame parity.
- **`[HW]`**:
  - Measure `MediaQualityAuditor` 3-anchor-frame verification overhead per segment on real hardware (< 30 ms per segment target).
- **`[MANUAL]`**:
  - Inspect the Segment Health Grid and SSIM/PSNR quality chart in the Svelte 5 / Flutter Media UI after a multi-node transcode.

## 6. Required Evidence & Artifacts
- `artifacts/stage-06/p2/sp3_chaos_transcode_recovery_report.json`
- `artifacts/stage-06/p2/sp3_quality_auditor_ssim_log.json`

## 7. Documentation, `TODO.md` & `CHANGELOG.md` Updates
- Document the 3-anchor perceptual quality check and checkpointed segment fault recovery in `docs/architecture/GPU_RENDER_AND_MEDIA.md` (Section 2.3).
- Mark `S6.P2.SP3` and Phase `S6.P2` complete in `TODO.md`; update `CHANGELOG.md`.

## 8. Rollback Criteria & Completion Gate
- **Rollback**: If a corrupted or frame-mismatched segment ever bypasses `MediaQualityAuditor` into the final stitched container.
- **Completion Gate**: Phase `S6.P2` completion gate passed (`S6_P2_SP1`..`SP3` green, >= 95% coverage). Proceed to Phase `S6.P3`.
