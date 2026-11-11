//! Automated Segment Structural & Perceptual Quality Auditor.
//!
//! Provides structural bitstream validation, exact frame count verification,
//! and 3-anchor frame (First IDR, Middle, Last) Y-channel SSIM/PSNR perceptual auditing.

use serde::{Deserialize, Serialize};

use crate::error::MediaError;
use crate::stitcher::TranscodedSegmentOutput;

/// Identification of anchor frame for perceptual quality evaluation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AnchorFrameType {
    FirstKeyframe,
    MiddleFrame,
    LastFrame,
}

/// Evaluation metrics for a sampled anchor frame.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnchorFrameEvaluation {
    pub anchor_type: AnchorFrameType,
    pub frame_idx: u64,
    pub ssim: f64,
    pub psnr_db: f64,
    pub passed: bool,
}

/// Policy governing acceptable structural and perceptual thresholds.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QualityPolicy {
    pub min_ssim_threshold: f64,
    pub min_psnr_threshold: f64,
    pub min_data_size_bytes: usize,
    pub max_bitrate_deviation_pct: f64,
}

impl Default for QualityPolicy {
    fn default() -> Self {
        Self {
            min_ssim_threshold: 0.92,
            min_psnr_threshold: 30.0,
            min_data_size_bytes: 1024,
            max_bitrate_deviation_pct: 50.0,
        }
    }
}

/// Comprehensive verification report produced for a single media segment.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SegmentQualityReport {
    pub segment_index: u32,
    pub passed: bool,
    pub expected_frames: u64,
    pub actual_frames: u64,
    pub has_idr_start: bool,
    pub avg_ssim: f64,
    pub min_ssim: f64,
    pub min_psnr_db: f64,
    pub anchor_evaluations: Vec<AnchorFrameEvaluation>,
    pub total_bytes: usize,
    pub rejection_reason: Option<String>,
}

/// Automated Structural and Perceptual Media Quality Auditor.
#[derive(Debug, Default)]
pub struct MediaQualityAuditor;

impl MediaQualityAuditor {
    /// Computes Structural Similarity Index (SSIM) between source and transcoded Y (luma) plane.
    #[must_use]
    pub fn compute_y_ssim(y_src: &[u8], y_dst: &[u8]) -> f64 {
        if y_src.is_empty() || y_src.len() != y_dst.len() {
            return 0.0;
        }

        let mut sum_x = 0u64;
        let mut sum_y = 0u64;
        let mut sum_xx = 0u64;
        let mut sum_yy = 0u64;
        let mut sum_xy = 0u64;

        for (&x, &y) in y_src.iter().zip(y_dst.iter()) {
            let ux = x as u64;
            let uy = y as u64;
            sum_x += ux;
            sum_y += uy;
            sum_xx += ux * ux;
            sum_yy += uy * uy;
            sum_xy += ux * uy;
        }

        let n = y_src.len() as f64;
        let mu_x = (sum_x as f64) / n;
        let mu_y = (sum_y as f64) / n;

        let var_x = ((sum_xx as f64) / n) - (mu_x * mu_x);
        let var_y = ((sum_yy as f64) / n) - (mu_y * mu_y);
        let cov_xy = ((sum_xy as f64) / n) - (mu_x * mu_y);

        // Standard SSIM stability constants for 8-bit dynamic range
        let c1 = 6.5025; // (0.01 * 255)^2
        let c2 = 58.5225; // (0.03 * 255)^2

        let numerator = (2.0 * mu_x * mu_y + c1) * (2.0 * cov_xy + c2);
        let denominator = (mu_x * mu_x + mu_y * mu_y + c1) * (var_x + var_y + c2);

        if denominator.abs() < 1e-9 {
            1.0
        } else {
            (numerator / denominator).clamp(0.0, 1.0)
        }
    }

    /// Computes Peak Signal-to-Noise Ratio (PSNR) in dB between source and transcoded Y plane.
    #[must_use]
    pub fn compute_y_psnr(y_src: &[u8], y_dst: &[u8]) -> f64 {
        if y_src.is_empty() || y_src.len() != y_dst.len() {
            return 0.0;
        }

        let mut sum_sq_err = 0u64;
        for (&x, &y) in y_src.iter().zip(y_dst.iter()) {
            let diff = (x as i32) - (y as i32);
            sum_sq_err += (diff * diff) as u64;
        }

        let n = y_src.len() as f64;
        let mse = (sum_sq_err as f64) / n;
        if mse < 1e-9 {
            100.0 // Mathematically identical
        } else {
            10.0 * (255.0 * 255.0 / mse).log10()
        }
    }

    /// Audits a transcoded segment against source anchor frames and structural invariants.
    pub fn verify_segment(
        segment_index: u32,
        expected_frames: u64,
        transcoded: &TranscodedSegmentOutput,
        source_anchors: &[(AnchorFrameType, u64, &[u8])], // (type, frame_idx, y_luma_bytes)
        transcoded_anchors: &[(AnchorFrameType, u64, &[u8])],
        policy: &QualityPolicy,
    ) -> Result<SegmentQualityReport, MediaError> {
        // 1. Structural checks
        // Check frame count parity
        if transcoded.actual_frame_count != expected_frames {
            let reason = format!(
                "structural frame count mismatch: expected {}, got {}",
                expected_frames, transcoded.actual_frame_count
            );
            return Ok(SegmentQualityReport {
                segment_index,
                passed: false,
                expected_frames,
                actual_frames: transcoded.actual_frame_count,
                has_idr_start: false,
                avg_ssim: 0.0,
                min_ssim: 0.0,
                min_psnr_db: 0.0,
                anchor_evaluations: Vec::new(),
                total_bytes: 0,
                rejection_reason: Some(reason),
            });
        }

        // Check first frame is IDR keyframe
        let has_idr = transcoded
            .packets
            .first()
            .is_some_and(|p| p.is_keyframe && p.stream_index == 0);

        if !has_idr {
            let reason = "segment bitstream does not start with an IDR keyframe".to_string();
            return Ok(SegmentQualityReport {
                segment_index,
                passed: false,
                expected_frames,
                actual_frames: transcoded.actual_frame_count,
                has_idr_start: false,
                avg_ssim: 0.0,
                min_ssim: 0.0,
                min_psnr_db: 0.0,
                anchor_evaluations: Vec::new(),
                total_bytes: 0,
                rejection_reason: Some(reason),
            });
        }

        // Total byte size sanity check
        let total_bytes: usize = transcoded.packets.iter().map(|p| p.data_size).sum();
        if total_bytes < policy.min_data_size_bytes {
            let reason = format!(
                "segment data size ({} bytes) below minimum threshold ({})",
                total_bytes, policy.min_data_size_bytes
            );
            return Ok(SegmentQualityReport {
                segment_index,
                passed: false,
                expected_frames,
                actual_frames: transcoded.actual_frame_count,
                has_idr_start: true,
                avg_ssim: 0.0,
                min_ssim: 0.0,
                min_psnr_db: 0.0,
                anchor_evaluations: Vec::new(),
                total_bytes,
                rejection_reason: Some(reason),
            });
        }

        // 2. Sampled Perceptual Quality Check (3 Anchors)
        let mut anchor_evals = Vec::new();
        let mut min_ssim = 1.0f64;
        let mut sum_ssim = 0.0f64;
        let mut min_psnr = 100.0f64;

        for (src_type, src_idx, src_y) in source_anchors {
            // Find corresponding transcoded anchor
            if let Some((_, _, dst_y)) = transcoded_anchors
                .iter()
                .find(|(t, idx, _)| t == src_type && idx == src_idx)
            {
                let ssim = Self::compute_y_ssim(src_y, dst_y);
                let psnr = Self::compute_y_psnr(src_y, dst_y);

                if ssim < min_ssim {
                    min_ssim = ssim;
                }
                if psnr < min_psnr {
                    min_psnr = psnr;
                }
                sum_ssim += ssim;

                let anchor_passed =
                    ssim >= policy.min_ssim_threshold && psnr >= policy.min_psnr_threshold;
                anchor_evals.push(AnchorFrameEvaluation {
                    anchor_type: *src_type,
                    frame_idx: *src_idx,
                    ssim,
                    psnr_db: psnr,
                    passed: anchor_passed,
                });
            }
        }

        let avg_ssim = if anchor_evals.is_empty() {
            1.0
        } else {
            sum_ssim / (anchor_evals.len() as f64)
        };

        let perceptual_passed = anchor_evals.is_empty()
            || (min_ssim >= policy.min_ssim_threshold && min_psnr >= policy.min_psnr_threshold);

        let rejection_reason = if !perceptual_passed {
            Some(format!(
                "perceptual quality below threshold: min SSIM {:.4} < {:.4} or min PSNR {:.2} dB < {:.2} dB",
                min_ssim, policy.min_ssim_threshold, min_psnr, policy.min_psnr_threshold
            ))
        } else {
            None
        };

        Ok(SegmentQualityReport {
            segment_index,
            passed: perceptual_passed,
            expected_frames,
            actual_frames: transcoded.actual_frame_count,
            has_idr_start: true,
            avg_ssim,
            min_ssim,
            min_psnr_db: min_psnr,
            anchor_evaluations: anchor_evals,
            total_bytes,
            rejection_reason,
        })
    }
}
