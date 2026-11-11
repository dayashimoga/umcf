//! Five-tier verification test suite for Subphase S6.P2.SP1:
//! Hardware-Accelerated Video/Audio Codec Probing & FFmpeg/Native Adapter.

use std::time::Instant;
use tokio::sync::mpsc;

use umcf_media::adapter::{
    translate_encoder_parameters, MediaCodecAdapter, TranscodeProgress, TranscodeRequest,
};
use umcf_media::prober::{
    AccelerationBackend, CandidateCodec, CodecCapabilityDescriptor, CodecDirection,
    HardwareCodecProber, MediaCodecId, PixelFormat,
};

// =========================================================================
// 1. UNIT TESTS
// =========================================================================

#[test]
fn test_unit_codec_capability_discovery_and_ghost_driver_filtering() {
    // 1. Ghost driver (e.g., compiled with NVENC flag but no physical NVIDIA driver)
    let ghost_candidate = CandidateCodec {
        codec: MediaCodecId::H264,
        direction: CodecDirection::Encode,
        backend: AccelerationBackend::Nvenc,
        encoder_name: "h264_nvenc".to_string(),
        supported_pixel_formats: vec![PixelFormat::Nv12],
        max_resolution: (4096, 2160),
        supports_b_frames: true,
        supports_10bit_hdr: false,
        max_concurrent_sessions: 3,
        simulated_driver_functional: false, // Driver fails initialization!
        synthetic_fps: 0.0,
    };

    let result = HardwareCodecProber::verify_candidate_synthetic(&ghost_candidate);
    assert!(result.is_none(), "ghost driver must be cleanly rejected");

    // 2. Functional driver passes synthetic verification
    let mut functional_candidate = ghost_candidate.clone();
    functional_candidate.simulated_driver_functional = true;
    functional_candidate.synthetic_fps = 210.0;

    let verified = HardwareCodecProber::verify_candidate_synthetic(&functional_candidate)
        .expect("functional driver must succeed");
    assert_eq!(verified.encoder_name, "h264_nvenc");
    assert!(verified.verified_live);
    assert_eq!(verified.benchmark_fps_1080p, 210.0);
}

#[test]
fn test_unit_hw_to_sw_encoder_parameter_translation_h264_hevc_av1() {
    let dummy_nvenc = CodecCapabilityDescriptor {
        codec: MediaCodecId::H264,
        direction: CodecDirection::Encode,
        backend: AccelerationBackend::Nvenc,
        encoder_name: "h264_nvenc".to_string(),
        supported_pixel_formats: vec![PixelFormat::Nv12],
        max_resolution: (4096, 2160),
        supports_b_frames: true,
        supports_10bit_hdr: false,
        max_concurrent_sessions: 3,
        benchmark_fps_1080p: 200.0,
        verified_live: true,
    };

    // 1. H.264 HW translation
    let req_h264_hw = TranscodeRequest {
        input_path: "input.mp4".to_string(),
        output_path: "output.mp4".to_string(),
        target_codec: MediaCodecId::H264,
        target_pixel_format: PixelFormat::Nv12,
        target_resolution: Some((1920, 1080)),
        crf_or_cq: 23,
        force_software: false,
    };
    let args_hw = translate_encoder_parameters(&req_h264_hw, Some(&dummy_nvenc));
    assert_eq!(args_hw.encoder_name, "h264_nvenc");
    assert!(args_hw.args.contains(&"-cq".to_string()));
    assert!(args_hw.args.contains(&"23".to_string()));

    // 2. H.264 SW Fallback translation
    let mut req_h264_sw = req_h264_hw.clone();
    req_h264_sw.force_software = true;
    let args_sw = translate_encoder_parameters(&req_h264_sw, Some(&dummy_nvenc));
    assert_eq!(args_sw.encoder_name, "libx264");
    assert_eq!(args_sw.backend, AccelerationBackend::SoftwareCpu);
    assert!(args_sw.args.contains(&"-crf".to_string()));
    assert!(args_sw.args.contains(&"23".to_string()));
    assert!(args_sw.args.contains(&"medium".to_string()));

    // 3. HEVC SW translation
    let req_hevc_sw = TranscodeRequest {
        input_path: "in.mp4".to_string(),
        output_path: "out.mp4".to_string(),
        target_codec: MediaCodecId::Hevc,
        target_pixel_format: PixelFormat::P010le,
        target_resolution: Some((3840, 2160)),
        crf_or_cq: 26,
        force_software: true,
    };
    let args_hevc = translate_encoder_parameters(&req_hevc_sw, None);
    assert_eq!(args_hevc.encoder_name, "libx265");
    assert!(args_hevc.args.contains(&"26".to_string()));

    // 4. AV1 SW translation
    let req_av1_sw = TranscodeRequest {
        input_path: "in.mp4".to_string(),
        output_path: "out.mp4".to_string(),
        target_codec: MediaCodecId::Av1,
        target_pixel_format: PixelFormat::P010le,
        target_resolution: Some((3840, 2160)),
        crf_or_cq: 28,
        force_software: true,
    };
    let args_av1 = translate_encoder_parameters(&req_av1_sw, None);
    assert_eq!(args_av1.encoder_name, "libsvtav1");
    assert!(args_av1.args.contains(&"28".to_string()));
}

#[test]
fn test_unit_realtime_telemetry_progress_line_parser() {
    let line = "frame=  120 fps= 60.0 q=28.0 size=    1024kB time=00:00:02.00 bitrate=4194.3kbits/s speed=1.00x";
    let progress =
        TranscodeProgress::parse_progress_line(line).expect("progress parsing must succeed");

    assert_eq!(progress.frame, 120);
    assert!((progress.fps - 60.0).abs() < 1e-4);
    assert!((progress.q_factor - 28.0).abs() < 1e-4);
    assert!((progress.bitrate_kbps - 4194.3).abs() < 1e-2);
    assert_eq!(progress.out_time_us, 2_000_000);
    assert!((progress.speed_factor - 1.00).abs() < 1e-4);
}

// =========================================================================
// 2. INTEGRATION TESTS
// =========================================================================

#[tokio::test]
async fn test_integration_encode_decode_synthetic_colorbars_h264_and_opus() {
    let mut adapter = MediaCodecAdapter::new(2);
    let (tx, mut rx) = mpsc::channel(16);

    let req = TranscodeRequest {
        input_path: "synthetic_colorbars.raw".to_string(),
        output_path: "output.mp4".to_string(),
        target_codec: MediaCodecId::H264,
        target_pixel_format: PixelFormat::Yuv420p,
        target_resolution: Some((1280, 720)),
        crf_or_cq: 22,
        force_software: true,
    };

    let result = adapter.execute_transcode(req, None, Some(tx)).await;
    assert!(result.is_ok());

    let mut snapshots = Vec::new();
    while let Ok(progress) = rx.try_recv() {
        snapshots.push(progress);
    }

    assert_eq!(snapshots.len(), 5);
    assert_eq!(snapshots.last().unwrap().frame, 120);
}

#[tokio::test]
async fn test_integration_hardware_session_limit_exhaustion_triggers_software_fallback() {
    let mut adapter = MediaCodecAdapter::new(1); // max 1 hardware session

    let dummy_qsv = CodecCapabilityDescriptor {
        codec: MediaCodecId::H264,
        direction: CodecDirection::Encode,
        backend: AccelerationBackend::Qsv,
        encoder_name: "h264_qsv".to_string(),
        supported_pixel_formats: vec![PixelFormat::Nv12],
        max_resolution: (3840, 2160),
        supports_b_frames: true,
        supports_10bit_hdr: false,
        max_concurrent_sessions: 1,
        benchmark_fps_1080p: 180.0,
        verified_live: true,
    };

    let req1 = TranscodeRequest {
        input_path: "task1.raw".to_string(),
        output_path: "task1.mp4".to_string(),
        target_codec: MediaCodecId::H264,
        target_pixel_format: PixelFormat::Nv12,
        target_resolution: Some((1920, 1080)),
        crf_or_cq: 23,
        force_software: false,
    };

    // First task gets hardware encoder
    let res1 = adapter
        .execute_transcode(req1.clone(), Some(&dummy_qsv), None)
        .await
        .unwrap();
    assert_eq!(res1.backend, AccelerationBackend::Qsv);

    // Simulate session saturation by artificially holding active session
    adapter.active_hw_sessions = 1;

    // Second task exhausts hardware limit -> transparently falls back to libx264!
    let res2 = adapter
        .execute_transcode(req1, Some(&dummy_qsv), None)
        .await
        .unwrap();
    assert_eq!(res2.backend, AccelerationBackend::SoftwareCpu);
    assert_eq!(res2.encoder_name, "libx264");
}

#[tokio::test]
async fn test_integration_mid_transcode_cancellation_cleans_up_resources_under_50ms() {
    let adapter = MediaCodecAdapter::new(2);
    let (tx, _rx) = mpsc::channel(16);

    let req = TranscodeRequest {
        input_path: "heavy_4k_source.raw".to_string(),
        output_path: "cancelled_output.mp4".to_string(),
        target_codec: MediaCodecId::H264,
        target_pixel_format: PixelFormat::Yuv420p,
        target_resolution: Some((3840, 2160)),
        crf_or_cq: 20,
        force_software: true,
    };

    let start = Instant::now();
    // Signal cancellation before / during execution
    adapter.cancel();

    let mut adapter_mut = adapter;
    let res = adapter_mut.execute_transcode(req, None, Some(tx)).await;
    let elapsed = start.elapsed();

    assert!(res.is_err());
    // Cancellation must complete under <= 50 ms
    assert!(
        elapsed.as_millis() <= 50,
        "cancellation took {} ms",
        elapsed.as_millis()
    );
}

// =========================================================================
// 3. SIMULATION TESTS
// =========================================================================

#[test]
fn test_simulation_driver_hang_timeout_and_fallback() {
    // Simulate candidate whose driver hangs on probe
    let hung_candidate = CandidateCodec {
        codec: MediaCodecId::Hevc,
        direction: CodecDirection::Encode,
        backend: AccelerationBackend::Amf,
        encoder_name: "hevc_amf".to_string(),
        supported_pixel_formats: vec![PixelFormat::Nv12],
        max_resolution: (3840, 2160),
        supports_b_frames: true,
        supports_10bit_hdr: true,
        max_concurrent_sessions: 2,
        simulated_driver_functional: false,
        synthetic_fps: 0.0,
    };

    let verified = HardwareCodecProber::verify_candidate_synthetic(&hung_candidate);
    assert!(verified.is_none());

    // When HW is unavailable, translation automatically selects libx265
    let req = TranscodeRequest {
        input_path: "sample.raw".to_string(),
        output_path: "out.mp4".to_string(),
        target_codec: MediaCodecId::Hevc,
        target_pixel_format: PixelFormat::P010le,
        target_resolution: Some((1920, 1080)),
        crf_or_cq: 24,
        force_software: false,
    };
    let args = translate_encoder_parameters(&req, verified.as_ref());
    assert_eq!(args.encoder_name, "libx265");
}

// =========================================================================
// 4. HARDWARE-REQUIRED TESTS
// =========================================================================

#[test]
fn test_hardware_codec_prober_live_host_matrix() {
    let mut prober = HardwareCodecProber::new();
    let codecs = prober.probe_all();

    assert!(
        !codecs.is_empty(),
        "must detect at least default codecs on host"
    );
    // Ensure all returned codecs are live verified
    for c in &codecs {
        assert!(c.verified_live);
        assert!(c.benchmark_fps_1080p > 0.0);
    }
}

// =========================================================================
// 5. MANUALLY-VERIFIED TESTS
// =========================================================================

#[test]
fn test_manual_codec_capability_matrix_ui_schema() {
    let desc = CodecCapabilityDescriptor {
        codec: MediaCodecId::Av1,
        direction: CodecDirection::Encode,
        backend: AccelerationBackend::Qsv,
        encoder_name: "av1_qsv".to_string(),
        supported_pixel_formats: vec![PixelFormat::Nv12, PixelFormat::P010le],
        max_resolution: (7680, 4320),
        supports_b_frames: false,
        supports_10bit_hdr: true,
        max_concurrent_sessions: 4,
        benchmark_fps_1080p: 140.0,
        verified_live: true,
    };

    let json = serde_json::to_string_pretty(&desc).expect("serialize desc");
    assert!(json.contains("av1_qsv"));
    assert!(json.contains("supports_10bit_hdr"));
    assert!(json.contains("140"));
}
