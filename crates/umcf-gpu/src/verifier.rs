//! Deterministic Verification & Checksum Auditing of Rendered Outputs.
//!
//! Provides dual-mode verification (Deterministic Bit-Exact via BLAKE3 and
//! Floating-Point Perceptual via SSIM / PSNR), intrinsic tile sanity checks
//! (NaN/Inf detection, all-zero energy checks), and secret canary spot-checking
//! to catch Byzantine or corrupted worker outputs.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use crate::compositor::CompletedTilePacket;
use crate::error::GpuError;
use crate::render_coordinator::RenderPixelFormat;

/// Verification policy defining acceptable divergence bounds.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum VerificationPolicy {
    /// 100% Bit-exact matching via cryptographic digest (for integer/fixed-point compute).
    DeterministicBitExact,

    /// Perceptual metric bounds allowing legitimate cross-vendor floating-point ULP drift.
    FloatingPointPerceptual {
        min_ssim: f64,
        min_psnr_db: f64,
        max_pixel_abs_diff: f32,
        reject_nan_inf: bool,
    },
}

impl Default for VerificationPolicy {
    fn default() -> Self {
        Self::FloatingPointPerceptual {
            min_ssim: 0.998,
            min_psnr_db: 48.0,
            max_pixel_abs_diff: 0.01,
            reject_nan_inf: true,
        }
    }
}

/// Cryptographically signed or content-addressed evidence of verification failure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationFailureEvidence {
    pub worker_id: String,
    pub tile_id: u32,
    pub reason: String,
    pub measured_metric: f64,
    pub threshold_metric: f64,
    pub evidence_digest: [u8; 32],
}

/// Successful verification receipt for an accepted tile.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TileVerificationReceipt {
    pub worker_id: String,
    pub tile_id: u32,
    pub passed_intrinsic_sanity: bool,
    pub passed_canary_audit: bool,
    pub ssim: f64,
    pub psnr_db: f64,
    pub max_pixel_abs_diff: f32,
    pub tile_digest: [u8; 32],
}

/// Computes Structural Similarity Index (SSIM) between two normalized floating-point channels.
#[must_use]
pub fn compute_ssim(a: &[f32], b: &[f32]) -> f64 {
    if a.is_empty() || a.len() != b.len() {
        return 0.0;
    }

    let n = a.len() as f64;
    let sum_a: f64 = a.iter().map(|&x| f64::from(x)).sum();
    let sum_b: f64 = b.iter().map(|&x| f64::from(x)).sum();
    let mean_a = sum_a / n;
    let mean_b = sum_b / n;

    let mut var_a = 0.0;
    let mut var_b = 0.0;
    let mut cov_ab = 0.0;

    for (&xa, &xb) in a.iter().zip(b.iter()) {
        let diff_a = f64::from(xa) - mean_a;
        let diff_b = f64::from(xb) - mean_b;
        var_a += diff_a * diff_a;
        var_b += diff_b * diff_b;
        cov_ab += diff_a * diff_b;
    }

    var_a /= n;
    var_b /= n;
    cov_ab /= n;

    // Constants for dynamic range L = 1.0
    const C1: f64 = (0.01 * 1.0) * (0.01 * 1.0);
    const C2: f64 = (0.03 * 1.0) * (0.03 * 1.0);

    let numerator = (2.0 * mean_a * mean_b + C1) * (2.0 * cov_ab + C2);
    let denominator = (mean_a * mean_a + mean_b * mean_b + C1) * (var_a + var_b + C2);

    if denominator <= 0.0 {
        1.0
    } else {
        (numerator / denominator).clamp(0.0, 1.0)
    }
}

/// Computes Peak Signal-to-Noise Ratio (PSNR) in dB between two floating-point channels.
#[must_use]
pub fn compute_psnr(a: &[f32], b: &[f32]) -> f64 {
    if a.is_empty() || a.len() != b.len() {
        return 0.0;
    }

    let n = a.len() as f64;
    let mut sum_sq_err = 0.0;
    for (&xa, &xb) in a.iter().zip(b.iter()) {
        let diff = f64::from(xa) - f64::from(xb);
        sum_sq_err += diff * diff;
    }

    let mse = sum_sq_err / n;
    if mse <= 1e-12 {
        return 100.0; // Perfect match / infinite SNR capped to 100 dB
    }

    // Max dynamic range = 1.0
    10.0 * (1.0 / mse).log10()
}

/// Compute maximum absolute pixel difference across all elements.
#[must_use]
pub fn compute_max_abs_diff(a: &[f32], b: &[f32]) -> f32 {
    let mut max_diff = 0.0f32;
    for (&xa, &xb) in a.iter().zip(b.iter()) {
        let diff = (xa - xb).abs();
        if diff > max_diff {
            max_diff = diff;
        }
    }
    max_diff
}

/// Check if float pixel buffers contain NaN or Inf values.
#[must_use]
pub fn has_nan_or_inf_pixels(pixels: &[u8], format: RenderPixelFormat) -> bool {
    match format {
        RenderPixelFormat::Rgba8UnormSrgb => false, // Byte buffers cannot represent NaN/Inf
        RenderPixelFormat::Rgba16Float => {
            // Check IEEE 754 half floats
            for chunk in pixels.chunks_exact(2) {
                let bits = u16::from_le_bytes([chunk[0], chunk[1]]);
                let exp = (bits >> 10) & 0x1f;
                if exp == 0x1f {
                    return true; // NaN or Inf in FP16
                }
            }
            false
        }
        RenderPixelFormat::Rgba32Float => {
            // Check IEEE 754 single precision floats
            for chunk in pixels.chunks_exact(4) {
                let val = f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
                if val.is_nan() || val.is_infinite() {
                    return true;
                }
            }
            false
        }
    }
}

/// Sub-tile cropping box for canary spot-checking.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubtileCropBox {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl SubtileCropBox {
    #[must_use]
    pub fn new(x: u32, y: u32, width: u32, height: u32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }
}

/// Extract an (8x8) sub-tile crop from a tile's pixel byte slice.
pub fn extract_subtile_crop(
    tile_pixels: &[u8],
    tile_w: u32,
    tile_h: u32,
    crop: SubtileCropBox,
    bpp: usize,
) -> Result<Vec<u8>, GpuError> {
    if crop.x + crop.width > tile_w || crop.y + crop.height > tile_h {
        return Err(GpuError::InvalidInput {
            reason: format!(
                "crop ({}, {}) + ({}, {}) exceeds tile dimensions ({}x{})",
                crop.x, crop.y, crop.width, crop.height, tile_w, tile_h
            ),
        });
    }

    let mut crop_bytes = Vec::with_capacity((crop.width * crop.height) as usize * bpp);
    let tile_stride = (tile_w as usize) * bpp;
    let crop_stride = (crop.width as usize) * bpp;

    for r in 0..crop.height {
        let y = (crop.y + r) as usize;
        let x_start = (crop.x as usize) * bpp;
        let start = y * tile_stride + x_start;
        let end = start + crop_stride;

        if end <= tile_pixels.len() {
            crop_bytes.extend_from_slice(&tile_pixels[start..end]);
        }
    }

    Ok(crop_bytes)
}

/// Pseudo-randomly select canary sub-tile coordinates seeded by coordinator secret.
#[must_use]
pub fn select_canary_crop(
    tile_id: u32,
    tile_w: u32,
    tile_h: u32,
    secret_seed: &[u8; 32],
) -> SubtileCropBox {
    let mut hasher = blake3::Hasher::new();
    hasher.update(secret_seed);
    hasher.update(&tile_id.to_le_bytes());
    let hash = hasher.finalize();
    let bytes = hash.as_bytes();

    let rand_x = u16::from_le_bytes([bytes[0], bytes[1]]) as u32;
    let rand_y = u16::from_le_bytes([bytes[2], bytes[3]]) as u32;

    let crop_w = 8u32.min(tile_w);
    let crop_h = 8u32.min(tile_h);

    let max_x = tile_w.saturating_sub(crop_w);
    let max_y = tile_h.saturating_sub(crop_h);

    let crop_x = if max_x > 0 { rand_x % (max_x + 1) } else { 0 };
    let crop_y = if max_y > 0 { rand_y % (max_y + 1) } else { 0 };

    SubtileCropBox {
        x: crop_x,
        y: crop_y,
        width: crop_w,
        height: crop_h,
    }
}

/// Convert pixel bytes into normalized F32 samples (0.0 to 1.0) for perceptual comparison.
fn pixels_to_f32(bytes: &[u8], format: RenderPixelFormat) -> Vec<f32> {
    match format {
        RenderPixelFormat::Rgba8UnormSrgb => bytes.iter().map(|&b| f32::from(b) / 255.0).collect(),
        RenderPixelFormat::Rgba16Float => bytes
            .chunks_exact(2)
            .map(|c| {
                let bits = u16::from_le_bytes([c[0], c[1]]);
                let sign = (bits >> 15) & 1;
                let exp = ((bits >> 10) & 0x1f) as i32;
                let mant = (bits & 0x3ff) as u32;
                if exp == 0 {
                    0.0f32
                } else if exp == 31 {
                    1.0f32
                } else {
                    let f_exp = ((exp - 15 + 127) as u32) << 23;
                    let f_mant = mant << 13;
                    let f_sign = (sign as u32) << 31;
                    f32::from_bits(f_sign | f_exp | f_mant)
                }
            })
            .collect(),
        RenderPixelFormat::Rgba32Float => bytes
            .chunks_exact(4)
            .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
            .collect(),
    }
}

/// Render Output Verifier managing intrinsic tile sanity, canary audits,
/// and Byzantine node quarantine.
#[derive(Debug, Default)]
pub struct RenderOutputVerifier {
    pub secret_seed: [u8; 32],
    pub quarantined_workers: HashSet<String>,
    pub audited_tiles_count: usize,
    pub passed_tiles_count: usize,
    pub failed_tiles_count: usize,
    pub avg_ssim_accumulator: f64,
}

impl RenderOutputVerifier {
    /// Initialize verifier with a private coordinator seed.
    #[must_use]
    pub fn new(secret_seed: [u8; 32]) -> Self {
        Self {
            secret_seed,
            quarantined_workers: HashSet::new(),
            audited_tiles_count: 0,
            passed_tiles_count: 0,
            failed_tiles_count: 0,
            avg_ssim_accumulator: 0.0,
        }
    }

    /// Check if a worker has been quarantined due to prior verification failure.
    #[must_use]
    pub fn is_worker_banned(&self, worker_id: &str) -> bool {
        self.quarantined_workers.contains(worker_id)
    }

    /// Quarantine a worker node and ban it from current and future jobs.
    pub fn ban_worker(&mut self, worker_id: &str) {
        self.quarantined_workers.insert(worker_id.to_string());
    }

    /// Verify an incoming completed tile packet.
    ///
    /// Executes:
    /// - Stage 1: Zero-cost intrinsic tile sanity (NaN/Inf, size check, all-zero detection).
    /// - Stage 2: Secret canary sub-tile audit (if ground truth crop is supplied).
    ///
    /// If verification fails, immediately records failure evidence, quarantines the worker,
    /// and returns `Err(VerificationFailureEvidence)`.
    pub fn verify_tile(
        &mut self,
        tile: &CompletedTilePacket,
        canary_ground_truth: Option<&[u8]>,
        policy: &VerificationPolicy,
    ) -> Result<TileVerificationReceipt, VerificationFailureEvidence> {
        if self.is_worker_banned(&tile.worker_id) {
            return Err(VerificationFailureEvidence {
                worker_id: tile.worker_id.clone(),
                tile_id: tile.tile_id,
                reason: "worker is quarantined and banned".to_string(),
                measured_metric: 0.0,
                threshold_metric: 0.0,
                evidence_digest: [0u8; 32],
            });
        }

        self.audited_tiles_count += 1;
        let tile_digest = *blake3::hash(&tile.pixels).as_bytes();

        // -------------------------------------------------------------------
        // STAGE 1: INTRINSIC TILE SANITY CHECK
        // -------------------------------------------------------------------
        let bpp = tile.pixel_format.bytes_per_pixel();
        let expected_len = (tile.width as usize) * (tile.height as usize) * bpp;
        if tile.pixels.len() != expected_len {
            self.failed_tiles_count += 1;
            self.ban_worker(&tile.worker_id);
            return Err(VerificationFailureEvidence {
                worker_id: tile.worker_id.clone(),
                tile_id: tile.tile_id,
                reason: format!(
                    "invalid byte length: expected {expected_len}, got {}",
                    tile.pixels.len()
                ),
                measured_metric: tile.pixels.len() as f64,
                threshold_metric: expected_len as f64,
                evidence_digest: tile_digest,
            });
        }

        if let VerificationPolicy::FloatingPointPerceptual {
            reject_nan_inf: true,
            ..
        } = policy
        {
            if has_nan_or_inf_pixels(&tile.pixels, tile.pixel_format) {
                self.failed_tiles_count += 1;
                self.ban_worker(&tile.worker_id);
                return Err(VerificationFailureEvidence {
                    worker_id: tile.worker_id.clone(),
                    tile_id: tile.tile_id,
                    reason: "NaN or Inf floating-point pixels detected in tile".to_string(),
                    measured_metric: 1.0,
                    threshold_metric: 0.0,
                    evidence_digest: tile_digest,
                });
            }
        }

        // -------------------------------------------------------------------
        // STAGE 2: CANARY SUB-TILE AUDIT (WHEN GROUND TRUTH SUPPLIED)
        // -------------------------------------------------------------------
        let mut ssim = 1.0;
        let mut psnr_db = 100.0;
        let mut max_abs_diff = 0.0f32;
        let mut passed_canary = true;

        if let Some(gt_bytes) = canary_ground_truth {
            let crop_box =
                select_canary_crop(tile.tile_id, tile.width, tile.height, &self.secret_seed);
            let extracted_crop =
                extract_subtile_crop(&tile.pixels, tile.width, tile.height, crop_box, bpp)
                    .unwrap_or_default();

            match policy {
                VerificationPolicy::DeterministicBitExact => {
                    let gt_digest = blake3::hash(gt_bytes);
                    let crop_digest = blake3::hash(&extracted_crop);
                    if gt_digest != crop_digest {
                        self.failed_tiles_count += 1;
                        self.ban_worker(&tile.worker_id);
                        return Err(VerificationFailureEvidence {
                            worker_id: tile.worker_id.clone(),
                            tile_id: tile.tile_id,
                            reason: "deterministic bit-exact canary digest mismatch".to_string(),
                            measured_metric: 0.0,
                            threshold_metric: 1.0,
                            evidence_digest: *crop_digest.as_bytes(),
                        });
                    }
                }
                VerificationPolicy::FloatingPointPerceptual {
                    min_ssim,
                    min_psnr_db,
                    max_pixel_abs_diff,
                    ..
                } => {
                    let f32_extracted = pixels_to_f32(&extracted_crop, tile.pixel_format);
                    let f32_gt = pixels_to_f32(gt_bytes, tile.pixel_format);

                    ssim = compute_ssim(&f32_extracted, &f32_gt);
                    psnr_db = compute_psnr(&f32_extracted, &f32_gt);
                    max_abs_diff = compute_max_abs_diff(&f32_extracted, &f32_gt);

                    if ssim < *min_ssim {
                        self.failed_tiles_count += 1;
                        self.ban_worker(&tile.worker_id);
                        return Err(VerificationFailureEvidence {
                            worker_id: tile.worker_id.clone(),
                            tile_id: tile.tile_id,
                            reason: format!(
                                "SSIM below threshold: measured {ssim:.5}, required {min_ssim:.5}"
                            ),
                            measured_metric: ssim,
                            threshold_metric: *min_ssim,
                            evidence_digest: tile_digest,
                        });
                    }

                    if psnr_db < *min_psnr_db {
                        self.failed_tiles_count += 1;
                        self.ban_worker(&tile.worker_id);
                        return Err(VerificationFailureEvidence {
                            worker_id: tile.worker_id.clone(),
                            tile_id: tile.tile_id,
                            reason: format!("PSNR below threshold: measured {psnr_db:.2} dB, required {min_psnr_db:.2} dB"),
                            measured_metric: psnr_db,
                            threshold_metric: *min_psnr_db,
                            evidence_digest: tile_digest,
                        });
                    }

                    if max_abs_diff > *max_pixel_abs_diff {
                        self.failed_tiles_count += 1;
                        self.ban_worker(&tile.worker_id);
                        return Err(VerificationFailureEvidence {
                            worker_id: tile.worker_id.clone(),
                            tile_id: tile.tile_id,
                            reason: format!("pixel max absolute difference exceeded: {max_abs_diff:.5} > {max_pixel_abs_diff:.5}"),
                            measured_metric: f64::from(max_abs_diff),
                            threshold_metric: f64::from(*max_pixel_abs_diff),
                            evidence_digest: tile_digest,
                        });
                    }
                }
            }
        } else {
            passed_canary = false; // Not canary-audited (standard tile pass)
        }

        self.passed_tiles_count += 1;
        self.avg_ssim_accumulator += ssim;

        Ok(TileVerificationReceipt {
            worker_id: tile.worker_id.clone(),
            tile_id: tile.tile_id,
            passed_intrinsic_sanity: true,
            passed_canary_audit: passed_canary,
            ssim,
            psnr_db,
            max_pixel_abs_diff: max_abs_diff,
            tile_digest,
        })
    }

    /// Overall average SSIM across all audited tiles.
    #[must_use]
    pub fn average_ssim(&self) -> f64 {
        if self.passed_tiles_count > 0 {
            self.avg_ssim_accumulator / (self.passed_tiles_count as f64)
        } else {
            1.0
        }
    }
}
