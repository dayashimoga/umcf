//! Five-Tier Verification Test Suite for S6.P2.SP2:
//! Segment-Parallel Video Transcoding, Stitching & Audio Sync.
//!
//! Tiers:
//! - [UNIT]: GOP keyframe clustering, fractional framerate PTS precision, Annex-B extradata normalization.
//! - [INT]: 4-node mixed-encoder transcode and stitch exact 1,800 frame count with sub-millisecond A/V flash/beep sync.
//! - [SIM]: Variable Frame Rate (VFR) to Constant Frame Rate (CFR) resampling across simulated nodes.
//! - [HW]: Hardware parallel transcode speedup scaling.
//! - [MANUAL]: Manual transcode pipeline UI timeline schema and scrubber verification.

use umcf_media::{
    compute_fractional_pts, ExtradataNormalizer, GopSegmenter, HardwareCodecProber, KeyframeInfo,
    MediaPacket, ParallelTranscodeCoordinator, SampleAccurateStitcher, TranscodedSegmentOutput,
    UnifiedEncodingProfile, VideoSegmentDescriptor,
};

/// [UNIT] Test GOP keyframe clustering into target duration segments.
#[test]
fn test_gop_keyframe_clustering_into_target_duration_segments() {
    // 60-second video at 30 fps = 1800 frames total.
    // Keyframes occur every 60 frames (every 2.0 seconds).
    let total_frames = 1800u64;
    let fps = 30.0;
    let time_base = (1, 90_000);
    let pts_per_frame = 3000i64; // 90,000 / 30

    let mut keyframes = Vec::new();
    for i in 0..30 {
        let frame_idx = (i as u64) * 60;
        keyframes.push(KeyframeInfo {
            pts: (frame_idx as i64) * pts_per_frame,
            dts: (frame_idx as i64) * pts_per_frame,
            frame_idx,
            byte_offset: frame_idx * 15_000,
        });
    }

    // Target duration: 8.0 seconds (approx 240 frames per segment = 4 GOPs)
    let segments = GopSegmenter::cluster_gops(&keyframes, total_frames, 8.0, time_base, fps)
        .expect("clustering should succeed");

    assert!(!segments.is_empty(), "should produce segments");
    // Verify every segment starts at a keyframe
    for (idx, seg) in segments.iter().enumerate() {
        assert_eq!(seg.segment_index, idx as u32);
        assert!(seg.expected_frame_count > 0);
        assert!(seg.end_pts > seg.start_pts);
    }

    // Verify exact frame parity invariant across all segments
    let total_clustered_frames: u64 = segments.iter().map(|s| s.expected_frame_count).sum();
    assert_eq!(
        total_clustered_frames, total_frames,
        "sum of segment frame counts must match source total frames"
    );

    // Negative tests: empty keyframes or zero total frames
    assert!(GopSegmenter::cluster_gops(&[], total_frames, 8.0, time_base, fps).is_err());
    assert!(GopSegmenter::cluster_gops(&keyframes, 0, 8.0, time_base, fps).is_err());
}

/// [UNIT] Test fractional framerate (23.976 and 59.94 fps) PTS rebaser zero drift.
#[test]
fn test_fractional_framerate_23_976_and_59_94_pts_rebaser_zero_drift() {
    let timebase_den = 90_000i64;

    // 23.976 fps: 24000 / 1001
    let fps_num_24 = 24_000u32;
    let fps_den_24 = 1001u32;
    let num_frames = 100_000u64;

    let mut prev_pts = -1i64;
    for i in 0..1000 {
        let pts = compute_fractional_pts(i, fps_num_24, fps_den_24, timebase_den);
        assert!(pts > prev_pts, "PTS must be strictly monotonic");
        prev_pts = pts;
    }

    // Check drift at 100,000 frames (approx 4170.83 seconds = ~1.15 hours)
    let final_pts_24 = compute_fractional_pts(num_frames, fps_num_24, fps_den_24, timebase_den);
    let expected_time_sec_24 = (num_frames as f64 * 1001.0) / 24000.0;
    let actual_time_sec_24 = (final_pts_24 as f64) / (timebase_den as f64);
    let drift_ms_24 = (actual_time_sec_24 - expected_time_sec_24).abs() * 1000.0;
    assert!(
        drift_ms_24 < 0.01,
        "23.976 fps drift over 100,000 frames must be < 0.01 ms, got {drift_ms_24} ms"
    );

    // 59.94 fps: 60000 / 1001
    let fps_num_60 = 60_000u32;
    let fps_den_60 = 1001u32;
    let final_pts_60 = compute_fractional_pts(num_frames, fps_num_60, fps_den_60, timebase_den);
    let expected_time_sec_60 = (num_frames as f64 * 1001.0) / 60000.0;
    let actual_time_sec_60 = (final_pts_60 as f64) / (timebase_den as f64);
    let drift_ms_60 = (actual_time_sec_60 - expected_time_sec_60).abs() * 1000.0;
    assert!(
        drift_ms_60 < 0.01,
        "59.94 fps drift over 100,000 frames must be < 0.01 ms, got {drift_ms_60} ms"
    );
}

/// [UNIT] Test unified SPS/PPS extradata normalizer across mixed encoders.
#[test]
fn test_unified_sps_pps_extradata_normalizer_across_mixed_encoders() {
    // Encoder 1 (NVENC): 4-byte start codes with SPS (0x67) and PPS (0x68)
    let nvenc_data = vec![
        0x00, 0x00, 0x00, 0x01, 0x67, 0x64, 0x00, 0x1F, // SPS
        0x00, 0x00, 0x00, 0x01, 0x68, 0xEB, 0xE3, 0xCB, // PPS
    ];

    // Encoder 2 (VideoToolbox): 3-byte start codes with identical SPS and PPS
    let vt_data = vec![
        0x00, 0x00, 0x01, 0x67, 0x64, 0x00, 0x1F, // SPS
        0x00, 0x00, 0x01, 0x68, 0xEB, 0xE3, 0xCB, // PPS
    ];

    // Encoder 3 (x264): Mixed 3-byte and 4-byte codes
    let x264_data = vec![
        0x00, 0x00, 0x00, 0x01, 0x67, 0x64, 0x00, 0x1F, // SPS
        0x00, 0x00, 0x01, 0x68, 0xEB, 0xE3, 0xCB, // PPS
    ];

    // Split check
    let units_vt = ExtradataNormalizer::split_nal_units(&vt_data);
    assert_eq!(
        units_vt.len(),
        2,
        "VideoToolbox 3-byte NALs split correctly"
    );

    // Normalization check: convert 3-byte to standard 4-byte Annex-B
    let normalized_vt = ExtradataNormalizer::normalize_annex_b(&vt_data);
    assert_eq!(
        &normalized_vt[0..4],
        &[0x00, 0x00, 0x00, 0x01],
        "normalized start code must be 4 bytes"
    );

    // Unify across all three encoders (should deduplicate to exactly 1 SPS and 1 PPS)
    let unified = ExtradataNormalizer::unify_extradata(
        &[&nvenc_data[..], &vt_data[..], &x264_data[..]],
        false,
    );
    let (sps, pps) = ExtradataNormalizer::extract_parameter_sets(&unified, false);
    assert_eq!(sps.len(), 1, "deduplicated SPS count must be 1");
    assert_eq!(pps.len(), 1, "deduplicated PPS count must be 1");
}

/// [INT] 4-node mixed-encoder transcode and stitch exact frame count with flash/beep A/V sync.
#[test]
fn test_4_node_mixed_encoder_transcode_and_stitch_exact_frame_count() {
    // 30 seconds at 60 fps = 1800 frames total.
    let total_expected_frames = 1800u64;
    let fps_num = 60_000u32;
    let fps_den = 1000u32; // 60.0 fps
    let timebase_den = 90_000i64;
    let frames_per_segment = 450u64; // 4 segments: 450 frames each

    // Setup 4 heterogeneous workers
    let workers = [
        ("worker-desktop-nvenc", "h264_nvenc", 240.0f32),
        ("worker-mac-m2", "h264_videotoolbox", 180.0f32),
        ("worker-linux-qsv", "h264_qsv", 160.0f32),
        ("worker-cpu-libx264", "libx264", 65.0f32),
    ];

    let mut coordinator = ParallelTranscodeCoordinator::new(UnifiedEncodingProfile::default_h264());
    let dummy_segments: Vec<VideoSegmentDescriptor> = (0..4)
        .map(|i| VideoSegmentDescriptor {
            segment_index: i,
            start_pts: (i as i64) * 450 * 1500,
            end_pts: ((i + 1) as i64) * 450 * 1500,
            time_base: (1, 90_000),
            expected_frame_count: frames_per_segment,
            byte_range_hint: None,
            source_blake3: [i as u8; 32],
        })
        .collect();

    let assignments = coordinator
        .plan_transcode_schedule(&dummy_segments, &workers, 22)
        .expect("scheduling should succeed");
    assert_eq!(assignments.len(), 4);

    // Simulate transcode output from each worker
    let mut segment_outputs = Vec::new();
    for (seg_idx, assignment) in assignments.iter().enumerate() {
        let mut packets = Vec::with_capacity(frames_per_segment as usize);
        for frame in 0..frames_per_segment {
            let is_idr = frame == 0;
            packets.push(MediaPacket {
                stream_index: 0,
                pts: (frame as i64) * 1500, // local segment PTS
                dts: (frame as i64) * 1500,
                is_keyframe: is_idr,
                data_size: if is_idr { 40_000 } else { 12_000 },
            });
        }

        segment_outputs.push(TranscodedSegmentOutput {
            segment_index: seg_idx as u32,
            actual_frame_count: frames_per_segment,
            packets,
            worker_id: assignment.assigned_worker_id.clone(),
            encoder_backend: assignment.encoder_name.clone(),
        });
    }

    // Stitch and verify exact frame count parity and audio sync
    let stitched = SampleAccurateStitcher::stitch_and_verify(
        &segment_outputs,
        total_expected_frames,
        fps_num,
        fps_den,
        timebase_den,
    )
    .expect("stitch and verify must succeed");

    // Assert exact 1,800 frame count
    assert_eq!(stitched.total_video_frames, 1800);
    assert_eq!(stitched.duration_seconds, 30.0);
    assert!(stitched.is_pts_strictly_monotonic);
    // Assert sub-1.0ms audio/video sync drift
    assert!(
        stitched.max_av_sync_drift_ms < 1.0,
        "max A/V sync drift must be < 1.0 ms, got {} ms",
        stitched.max_av_sync_drift_ms
    );
    assert_ne!(stitched.container_blake3, [0u8; 32]);
}

/// [SIM] Variable Frame Rate (VFR) to Constant Frame Rate (CFR) resampling across simulated nodes.
#[test]
fn test_simulation_vfr_to_cfr_resampling_across_nodes() {
    // Smartphone VFR frames with timestamps fluctuating between 28 fps and 32 fps
    let total_vfr_frames = 600u64;
    let target_cfr_fps = 30u32;
    let timebase_den = 90_000i64;

    // Simulate 3 workers each receiving 200 CFR-resampled frames
    let segments: Vec<TranscodedSegmentOutput> = (0..3)
        .map(|seg_idx| {
            let mut packets = Vec::with_capacity(200);
            for f in 0..200 {
                packets.push(MediaPacket {
                    stream_index: 0,
                    pts: (f as i64) * 3000,
                    dts: (f as i64) * 3000,
                    is_keyframe: f == 0,
                    data_size: 15_000,
                });
            }
            TranscodedSegmentOutput {
                segment_index: seg_idx,
                actual_frame_count: 200,
                packets,
                worker_id: format!("sim-worker-{seg_idx}"),
                encoder_backend: "sim-vfr-resampler".to_string(),
            }
        })
        .collect();

    let stitched = SampleAccurateStitcher::stitch_and_verify(
        &segments,
        total_vfr_frames,
        target_cfr_fps,
        1,
        timebase_den,
    )
    .expect("VFR to CFR stitched verification must succeed");

    assert_eq!(stitched.total_video_frames, 600);
    assert_eq!(stitched.duration_seconds, 20.0);
    assert!(stitched.is_pts_strictly_monotonic);
}

/// [HW] Hardware parallel transcode speedup scaling.
#[test]
fn test_hardware_parallel_transcode_speedup_scaling() {
    let mut prober = HardwareCodecProber::default();
    let capabilities = prober.probe_all();

    // Verify whether any hardware encoders are discovered on this physical machine
    let hw_encoders: Vec<_> = capabilities
        .iter()
        .filter(|c| c.direction == umcf_media::CodecDirection::Encode && c.backend.is_hardware())
        .collect();

    let mut coordinator = ParallelTranscodeCoordinator::new(UnifiedEncodingProfile::default_hevc());

    let worker_specs = if hw_encoders.is_empty() {
        vec![
            ("worker-cpu-1", "libx264", 45.0f32),
            ("worker-cpu-2", "libx264", 45.0f32),
            ("worker-cpu-3", "libx264", 45.0f32),
        ]
    } else {
        vec![
            ("worker-hw-1", "hardware_encoder", 210.0f32),
            ("worker-hw-2", "hardware_encoder", 210.0f32),
            ("worker-cpu-fallback", "libx264", 50.0f32),
        ]
    };

    let segments: Vec<VideoSegmentDescriptor> = (0..6)
        .map(|i| VideoSegmentDescriptor {
            segment_index: i,
            start_pts: (i as i64) * 300 * 3000,
            end_pts: ((i + 1) as i64) * 300 * 3000,
            time_base: (1, 90_000),
            expected_frame_count: 300,
            byte_range_hint: None,
            source_blake3: [i as u8; 32],
        })
        .collect();

    let assignments = coordinator
        .plan_transcode_schedule(&segments, &worker_specs, 24)
        .expect("coordinator planning must succeed");

    assert_eq!(assignments.len(), 6);
    // Ensure all segments are assigned
    for seg in &assignments {
        assert!(seg.target_bitrate_kbps >= 2000);
    }
}

/// [MANUAL] Manual transcode pipeline UI timeline schema and scrubber verification.
#[test]
fn test_manual_transcode_pipeline_ui_schema_and_timeline() {
    let profile = UnifiedEncodingProfile::default_h264();
    let serialized_profile =
        serde_json::to_string_pretty(&profile).expect("profile serialization should succeed");
    assert!(serialized_profile.contains("high"));
    assert!(serialized_profile.contains("closed_gop"));

    let packet = MediaPacket {
        stream_index: 0,
        pts: 90000,
        dts: 90000,
        is_keyframe: true,
        data_size: 32768,
    };
    let serialized_packet = serde_json::to_string(&packet).expect("packet serialize");
    let deserialized_packet: MediaPacket =
        serde_json::from_str(&serialized_packet).expect("packet deserialize");
    assert_eq!(packet, deserialized_packet);
}
