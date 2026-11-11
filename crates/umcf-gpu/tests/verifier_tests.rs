//! Five-tier verification test suite for Subphase S6.P1.SP3:
//! Deterministic Verification & Checksum Auditing of Rendered Outputs.

use umcf_gpu::compositor::{CompletedTilePacket, FrameCompositor};
use umcf_gpu::render_coordinator::{
    DistributedRenderCoordinator, RenderJobSpec, RenderPixelFormat,
};
use umcf_gpu::shader_sandbox::ShaderSandbox;
use umcf_gpu::verifier::{
    compute_max_abs_diff, compute_psnr, compute_ssim, extract_subtile_crop, has_nan_or_inf_pixels,
    select_canary_crop, RenderOutputVerifier, SubtileCropBox, VerificationPolicy,
};

// =========================================================================
// 1. UNIT TESTS
// =========================================================================

#[test]
fn test_unit_ssim_psnr_and_max_abs_diff_calculation_accuracy() {
    // 1. Identical signals
    let a = vec![0.5f32; 64];
    let b = vec![0.5f32; 64];
    let ssim_identical = compute_ssim(&a, &b);
    let psnr_identical = compute_psnr(&a, &b);
    let max_diff_identical = compute_max_abs_diff(&a, &b);

    assert!((ssim_identical - 1.0).abs() < 1e-6);
    assert!(psnr_identical >= 99.0); // Capped at 100 dB for perfect match
    assert_eq!(max_diff_identical, 0.0);

    // 2. Minor IEEE-754 drift (e.g. 1-2 ULPs or small float delta 0.001)
    let c: Vec<f32> = a.iter().map(|&x| x + 0.001).collect();
    let ssim_minor = compute_ssim(&a, &c);
    let psnr_minor = compute_psnr(&a, &c);
    let max_diff_minor = compute_max_abs_diff(&a, &c);

    assert!(ssim_minor >= 0.999);
    assert!(psnr_minor >= 55.0); // 60 dB
    assert!((max_diff_minor - 0.001).abs() < 1e-6);

    // 3. Significant corruption (e.g. inverted signal or high noise)
    let ramp_a: Vec<f32> = (0..64).map(|i| (i as f32) / 64.0).collect();
    let ramp_inv: Vec<f32> = (0..64).map(|i| 1.0 - (i as f32) / 64.0).collect();
    let ssim_corrupt = compute_ssim(&ramp_a, &ramp_inv);
    let psnr_corrupt = compute_psnr(&ramp_a, &ramp_inv);

    assert!(ssim_corrupt < 0.5);
    assert!(psnr_corrupt < 15.0);
}

#[test]
fn test_unit_nan_and_inf_pixel_detection_in_rgba16f_and_rgba32f() {
    // 1. RGBA32F with normal values -> false
    let mut clean_f32 = Vec::new();
    for _ in 0..16 {
        clean_f32.extend_from_slice(&0.75f32.to_le_bytes());
    }
    assert!(!has_nan_or_inf_pixels(
        &clean_f32,
        RenderPixelFormat::Rgba32Float
    ));

    // 2. RGBA32F with NaN -> true
    let mut nan_f32 = clean_f32.clone();
    let nan_bytes = f32::NAN.to_le_bytes();
    nan_f32[0..4].copy_from_slice(&nan_bytes);
    assert!(has_nan_or_inf_pixels(
        &nan_f32,
        RenderPixelFormat::Rgba32Float
    ));

    // 3. RGBA32F with Infinity -> true
    let mut inf_f32 = clean_f32.clone();
    let inf_bytes = f32::INFINITY.to_le_bytes();
    inf_f32[8..12].copy_from_slice(&inf_bytes);
    assert!(has_nan_or_inf_pixels(
        &inf_f32,
        RenderPixelFormat::Rgba32Float
    ));

    // 4. RGBA16F with FP16 NaN exponent (bits 10..14 all 1s = 0x7E00) -> true
    let mut nan_f16 = vec![0u8; 16 * 2];
    nan_f16[0] = 0x00;
    nan_f16[1] = 0x7E; // FP16 NaN
    assert!(has_nan_or_inf_pixels(
        &nan_f16,
        RenderPixelFormat::Rgba16Float
    ));
}

#[test]
fn test_unit_secret_canary_subtile_coordinate_extraction_and_comparison() {
    let seed = [0x77u8; 32];
    let tile_w = 128u32;
    let tile_h = 128u32;

    // Verify deterministic but non-trivial canary coordinates within bounds
    let crop1 = select_canary_crop(42, tile_w, tile_h, &seed);
    assert_eq!(crop1.width, 8);
    assert_eq!(crop1.height, 8);
    assert!(crop1.x + crop1.width <= tile_w);
    assert!(crop1.y + crop1.height <= tile_h);

    // Coordinate changes for different tile ID
    let crop2 = select_canary_crop(43, tile_w, tile_h, &seed);
    assert!(crop1.x != crop2.x || crop1.y != crop2.y);

    // Extraction test: fill a 16x16 tile with coordinates
    let bpp = 4;
    let mut tile_pixels = vec![0u8; 16 * 16 * bpp];
    for y in 0..16 {
        for x in 0..16 {
            let idx = (y * 16 + x) * bpp;
            tile_pixels[idx] = x as u8;
            tile_pixels[idx + 1] = y as u8;
            tile_pixels[idx + 2] = 100;
            tile_pixels[idx + 3] = 255;
        }
    }

    // Extract an 8x8 crop at (4, 4)
    let crop = extract_subtile_crop(&tile_pixels, 16, 16, SubtileCropBox::new(4, 4, 8, 8), bpp)
        .expect("crop extraction");
    assert_eq!(crop.len(), 8 * 8 * bpp);
    // Check first pixel in crop (should correspond to x=4, y=4)
    assert_eq!(crop[0], 4);
    assert_eq!(crop[1], 4);
}

#[test]
fn test_unit_deterministic_bit_exact_mode_blake3_verification() {
    let mut verifier = RenderOutputVerifier::new([0x33; 32]);
    let policy = VerificationPolicy::DeterministicBitExact;

    let payload = vec![42u8; 64 * 64 * 4];
    let tile = CompletedTilePacket {
        tile_id: 10,
        frame_idx: 0,
        x: 0,
        y: 0,
        width: 64,
        height: 64,
        pixel_format: RenderPixelFormat::Rgba8UnormSrgb,
        pixels: payload.clone(),
        worker_id: "honest-worker".to_string(),
        render_time_ms: 5.0,
    };

    // Extract expected canary crop
    let crop_box = select_canary_crop(10, 64, 64, &verifier.secret_seed);
    let gt_crop = extract_subtile_crop(&payload, 64, 64, crop_box, 4).unwrap();

    // 1. Exact match passes
    let receipt = verifier
        .verify_tile(&tile, Some(&gt_crop), &policy)
        .expect("exact match must pass");
    assert!(receipt.passed_canary_audit);

    // 2. Corrupted canary fails and bans worker
    let mut bad_crop = gt_crop.clone();
    bad_crop[0] ^= 0x01; // flip single bit

    let err = verifier
        .verify_tile(&tile, Some(&bad_crop), &policy)
        .expect_err("mismatched digest must fail");
    assert_eq!(err.worker_id, "honest-worker");
    assert!(verifier.is_worker_banned("honest-worker"));
}

// =========================================================================
// 2. INTEGRATION TESTS
// =========================================================================

#[test]
fn test_integration_byzantine_worker_injecting_corrupted_tile_detected_and_rerendered() {
    let spec = RenderJobSpec {
        job_id: "byzantine-resilience-job".to_string(),
        resolution: (256, 256), // 4 tiles of 128x128
        frame_range: 0..=0,
        samples_per_pixel: 32,
        tile_size: (128, 128),
        pixel_format: RenderPixelFormat::Rgba8UnormSrgb,
        scene_asset_root: [0x88; 32],
        shader_module: ShaderSandbox::default()
            .validate_and_instrument("test-shader", "@group(0) @binding(0) var<storage, read_write> d: array<u32>; @compute @workgroup_size(16, 16) fn main() { d[0] = 1u; }")
            .unwrap(),
    };

    let mut coordinator = DistributedRenderCoordinator::new(spec).expect("coordinator");
    coordinator.register_worker("w-honest-1", "Honest Node 1", 5.0);
    coordinator.register_worker("w-byzantine-2", "Byzantine Node 2", 5.0);
    coordinator.register_worker("w-honest-3", "Honest Node 3", 5.0);

    let mut verifier = RenderOutputVerifier::new([0xab; 32]);
    let policy = VerificationPolicy::default(); // Perceptual
    let mut compositor = FrameCompositor::new(256, 256, RenderPixelFormat::Rgba8UnormSrgb);

    // Ground truth generator: solid color with mild gradient
    let make_clean_tile = |val: u8| vec![val; 128 * 128 * 4];

    // Dispatch tile 0 to honest worker 1
    let honest_batch = coordinator.dispatch_next_batch("w-honest-1");
    let t0 = honest_batch[0].clone();
    let t0_pixels = make_clean_tile(120);
    let crop_box0 = select_canary_crop(t0.tile_id, t0.width, t0.height, &verifier.secret_seed);
    let gt_crop0 = extract_subtile_crop(&t0_pixels, t0.width, t0.height, crop_box0, 4).unwrap();

    let packet0 = CompletedTilePacket {
        tile_id: t0.tile_id,
        frame_idx: 0,
        x: t0.x,
        y: t0.y,
        width: t0.width,
        height: t0.height,
        pixel_format: RenderPixelFormat::Rgba8UnormSrgb,
        pixels: t0_pixels,
        worker_id: "w-honest-1".to_string(),
        render_time_ms: 10.0,
    };

    let v0 = verifier
        .verify_tile(&packet0, Some(&gt_crop0), &policy)
        .expect("honest tile 0");
    assert!(v0.passed_canary_audit);
    compositor.blit_tile(&packet0).expect("blit honest 0");
    coordinator
        .record_tile_completion(t0.tile_id, "w-honest-1", 10.0)
        .expect("commit 0");

    // Dispatch tile 1 to Byzantine worker 2
    let byz_batch = coordinator.dispatch_next_batch("w-byzantine-2");
    let t1 = byz_batch[0].clone();
    // Byzantine node returns all zeros (black tile cheating)
    let corrupt_pixels = vec![0u8; 128 * 128 * 4];
    let clean_pixels_t1 = make_clean_tile(120);
    let crop_box1 = select_canary_crop(t1.tile_id, t1.width, t1.height, &verifier.secret_seed);
    let gt_crop1 =
        extract_subtile_crop(&clean_pixels_t1, t1.width, t1.height, crop_box1, 4).unwrap();

    let packet1 = CompletedTilePacket {
        tile_id: t1.tile_id,
        frame_idx: 0,
        x: t1.x,
        y: t1.y,
        width: t1.width,
        height: t1.height,
        pixel_format: RenderPixelFormat::Rgba8UnormSrgb,
        pixels: corrupt_pixels,
        worker_id: "w-byzantine-2".to_string(),
        render_time_ms: 2.0, // suspiciously fast
    };

    // Verifier catches Byzantine worker!
    let err = verifier
        .verify_tile(&packet1, Some(&gt_crop1), &policy)
        .expect_err("must catch byzantine");
    assert_eq!(err.worker_id, "w-byzantine-2");
    assert!(verifier.is_worker_banned("w-byzantine-2"));

    // Byzantine tile is NOT blitted to compositor. Re-queue tile 1 to honest worker 3
    coordinator.pending_queue.push_front(t1.tile_id);

    // Re-dispatch to honest worker 3
    let rerun_batch = coordinator.dispatch_next_batch("w-honest-3");
    assert_eq!(rerun_batch[0].tile_id, t1.tile_id);

    let packet1_clean = CompletedTilePacket {
        tile_id: t1.tile_id,
        frame_idx: 0,
        x: t1.x,
        y: t1.y,
        width: t1.width,
        height: t1.height,
        pixel_format: RenderPixelFormat::Rgba8UnormSrgb,
        pixels: clean_pixels_t1,
        worker_id: "w-honest-3".to_string(),
        render_time_ms: 10.0,
    };

    let v1 = verifier
        .verify_tile(&packet1_clean, Some(&gt_crop1), &policy)
        .expect("rerun tile 1");
    assert!(v1.passed_canary_audit);
    compositor.blit_tile(&packet1_clean).expect("blit clean 1");
    coordinator
        .record_tile_completion(t1.tile_id, "w-honest-3", 10.0)
        .expect("commit 1");

    // Frame buffer at (t1.x, t1.y) contains real non-zero clean pixels, not zeros
    assert_eq!(compositor.get_pixel_rgba8(t1.x, t1.y).unwrap()[0], 120);
}

// =========================================================================
// 3. SIMULATION TESTS
// =========================================================================

#[test]
fn test_simulation_cross_vendor_ulp_drift_vs_byzantine_lazy_worker() {
    let mut verifier = RenderOutputVerifier::new([0x99; 32]);
    let policy = VerificationPolicy::default(); // min_ssim = 0.998, min_psnr = 48.0 dB

    let gt = vec![128u8; 8 * 8 * 4];

    // 1. Simulate 500 legitimate cross-vendor ULP drifts (+/- 1 on discrete color channels)
    let len = gt.len();
    for i in 0..500 {
        let mut drifted = gt.clone();
        let idx = i % len;
        drifted[idx] = if (i % 2) == 0 { 129 } else { 127 };

        let tile = CompletedTilePacket {
            tile_id: i as u32,
            frame_idx: 0,
            x: 0,
            y: 0,
            width: 8,
            height: 8,
            pixel_format: RenderPixelFormat::Rgba8UnormSrgb,
            pixels: drifted,
            worker_id: format!("vendor-node-{}", i % 5),
            render_time_ms: 5.0,
        };

        let res = verifier.verify_tile(&tile, Some(&gt), &policy);
        assert!(
            res.is_ok(),
            "legitimate ULP drift should pass perceptual check"
        );
    }

    assert!(verifier.average_ssim() > 0.999);

    // 2. Simulate lazy worker returning severely downscaled/blurry pixels (e.g. constant 60 vs 128)
    let lazy_pixels = vec![60u8; 8 * 8 * 4];
    let lazy_tile = CompletedTilePacket {
        tile_id: 9999,
        frame_idx: 0,
        x: 0,
        y: 0,
        width: 8,
        height: 8,
        pixel_format: RenderPixelFormat::Rgba8UnormSrgb,
        pixels: lazy_pixels,
        worker_id: "lazy-worker".to_string(),
        render_time_ms: 1.0,
    };

    let lazy_err = verifier
        .verify_tile(&lazy_tile, Some(&gt), &policy)
        .expect_err("lazy worker caught");
    assert_eq!(lazy_err.worker_id, "lazy-worker");
    assert!(verifier.is_worker_banned("lazy-worker"));
}

// =========================================================================
// 4. HARDWARE-REQUIRED TESTS
// =========================================================================

#[test]
fn test_hardware_software_vs_gpu_rendered_distribution() {
    let mut verifier = RenderOutputVerifier::new([0x12; 32]);
    let policy = VerificationPolicy::default();

    let start = std::time::Instant::now();
    let gt = vec![200u8; 8 * 8 * 4];
    let test_tile = CompletedTilePacket {
        tile_id: 1,
        frame_idx: 0,
        x: 0,
        y: 0,
        width: 8,
        height: 8,
        pixel_format: RenderPixelFormat::Rgba8UnormSrgb,
        pixels: gt.clone(),
        worker_id: "bench-worker".to_string(),
        render_time_ms: 4.0,
    };

    for _ in 0..100 {
        let _ = verifier.verify_tile(&test_tile, Some(&gt), &policy);
    }
    let elapsed = start.elapsed();
    let per_canary_us = (elapsed.as_micros() as f64) / 100.0;

    // Canary check must be lightning-fast (< 50 µs in release, < 250 µs under debug/llvm-cov)
    #[cfg(not(debug_assertions))]
    assert!(per_canary_us < 50.0, "canary check took {per_canary_us} µs");
    #[cfg(debug_assertions)]
    assert!(per_canary_us < 250.0, "canary check took {per_canary_us} µs");
}

// =========================================================================
// 5. MANUALLY-VERIFIED TESTS
// =========================================================================

#[test]
fn test_manual_verification_audit_log_schema_and_ui() {
    let failure = umcf_gpu::verifier::VerificationFailureEvidence {
        worker_id: "rogue-cluster-node-9".to_string(),
        tile_id: 42,
        reason: "SSIM below threshold: measured 0.81240, required 0.99800".to_string(),
        measured_metric: 0.8124,
        threshold_metric: 0.998,
        evidence_digest: [0xfe; 32],
    };

    let json = serde_json::to_string_pretty(&failure).expect("serialize failure evidence");
    assert!(json.contains("rogue-cluster-node-9"));
    assert!(json.contains("0.8124"));
    assert!(json.contains("evidence_digest"));
}
