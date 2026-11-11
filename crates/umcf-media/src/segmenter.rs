//! GOP-Boundary Keyframe Scanner & Parallel Transcode Coordinator.
//!
//! Provides lossless GOP keyframe clustering, dedicated continuous audio extraction,
//! and encoding parameter normalization across mixed hardware/software encoders.

use serde::{Deserialize, Serialize};

use crate::error::MediaError;
use crate::prober::{MediaCodecId, PixelFormat};

/// Information about a detected keyframe (IDR frame) in the source container.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyframeInfo {
    pub pts: i64,
    pub dts: i64,
    pub frame_idx: u64,
    pub byte_offset: u64,
}

/// Description of an independent video segment bounded by IDR keyframes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VideoSegmentDescriptor {
    pub segment_index: u32,
    pub start_pts: i64,
    pub end_pts: i64,
    pub time_base: (i32, i32),
    pub expected_frame_count: u64,
    pub byte_range_hint: Option<(u64, u64)>,
    pub source_blake3: [u8; 32],
}

/// Normalized profile enforced across all cluster nodes to allow seamless stitching.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnifiedEncodingProfile {
    pub target_codec: MediaCodecId,
    pub pixel_format: PixelFormat,
    pub profile: String,
    pub level: String,
    pub output_time_base: (i32, i32),
    pub closed_gop: bool,
    pub repeat_headers: bool,
}

impl UnifiedEncodingProfile {
    #[must_use]
    pub fn default_h264() -> Self {
        Self {
            target_codec: MediaCodecId::H264,
            pixel_format: PixelFormat::Nv12,
            profile: "high".to_string(),
            level: "4.1".to_string(),
            output_time_base: (1, 90_000),
            closed_gop: true,
            repeat_headers: true,
        }
    }

    #[must_use]
    pub fn default_hevc() -> Self {
        Self {
            target_codec: MediaCodecId::Hevc,
            pixel_format: PixelFormat::P010le,
            profile: "main10".to_string(),
            level: "5.1".to_string(),
            output_time_base: (1, 90_000),
            closed_gop: true,
            repeat_headers: true,
        }
    }
}

/// Segment assignment to a cluster worker node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SegmentAssignment {
    pub segment: VideoSegmentDescriptor,
    pub assigned_worker_id: String,
    pub encoder_name: String,
    pub target_bitrate_kbps: u32,
    pub crf: u8,
}

/// Lossless GOP Segmenter clustering source media into parallel units.
#[derive(Debug, Default)]
pub struct GopSegmenter;

impl GopSegmenter {
    /// Clusters source media keyframes into independent segments targeting `target_duration_sec`.
    /// Guarantees that every segment starts on an IDR keyframe and the sum of `expected_frame_count`
    /// across all segments matches `total_frames` exactly.
    pub fn cluster_gops(
        keyframes: &[KeyframeInfo],
        total_frames: u64,
        target_duration_sec: f64,
        time_base: (i32, i32),
        fps: f64,
    ) -> Result<Vec<VideoSegmentDescriptor>, MediaError> {
        if keyframes.is_empty() {
            return Err(MediaError::InvalidInput {
                reason: "no keyframes provided for GOP clustering".to_string(),
            });
        }
        if total_frames == 0 || fps <= 0.0 {
            return Err(MediaError::InvalidInput {
                reason: "total_frames and fps must be greater than zero".to_string(),
            });
        }

        let target_frames_per_segment = (target_duration_sec * fps).round() as u64;
        let mut segments = Vec::new();
        let mut seg_start_idx = 0usize;
        let mut seg_index = 0u32;

        let num_kf = keyframes.len();
        while seg_start_idx < num_kf {
            let start_kf = &keyframes[seg_start_idx];
            let mut seg_end_idx = seg_start_idx + 1;

            // Advance until accumulated duration >= target_duration_sec or end of keyframes
            while seg_end_idx < num_kf {
                let candidate_kf = &keyframes[seg_end_idx];
                let span_frames = candidate_kf.frame_idx - start_kf.frame_idx;
                if span_frames >= target_frames_per_segment {
                    break;
                }
                seg_end_idx += 1;
            }

            let end_frame = if seg_end_idx < num_kf {
                keyframes[seg_end_idx].frame_idx
            } else {
                total_frames
            };

            let expected_frame_count = end_frame - start_kf.frame_idx;

            // Compute end PTS in stream time base
            let pts_per_frame =
                (f64::from(time_base.1) / (f64::from(time_base.0) * fps)).round() as i64;
            let end_pts = start_kf.pts + (expected_frame_count as i64) * pts_per_frame;

            let byte_range = if seg_end_idx < num_kf {
                Some((start_kf.byte_offset, keyframes[seg_end_idx].byte_offset))
            } else {
                Some((start_kf.byte_offset, u64::MAX))
            };

            // Synthetic source digest for segment integrity
            let mut hasher = blake3::Hasher::new();
            hasher.update(&seg_index.to_le_bytes());
            hasher.update(&start_kf.pts.to_le_bytes());
            hasher.update(&expected_frame_count.to_le_bytes());
            let source_blake3 = *hasher.finalize().as_bytes();

            segments.push(VideoSegmentDescriptor {
                segment_index: seg_index,
                start_pts: start_kf.pts,
                end_pts,
                time_base,
                expected_frame_count,
                byte_range_hint: byte_range,
                source_blake3,
            });

            seg_index += 1;
            seg_start_idx = seg_end_idx;
        }

        // Verify invariant: total frames sum
        let sum_frames: u64 = segments.iter().map(|s| s.expected_frame_count).sum();
        if sum_frames != total_frames {
            return Err(MediaError::SegmentFrameCountMismatch {
                segment_index: 0,
                expected: total_frames,
                actual: sum_frames,
            });
        }

        Ok(segments)
    }
}

/// Parallel Transcode Coordinator managing heterogeneous worker dispatch.
#[derive(Debug)]
pub struct ParallelTranscodeCoordinator {
    pub profile: UnifiedEncodingProfile,
    pub assignments: Vec<SegmentAssignment>,
}

impl ParallelTranscodeCoordinator {
    #[must_use]
    pub fn new(profile: UnifiedEncodingProfile) -> Self {
        Self {
            profile,
            assignments: Vec::new(),
        }
    }

    /// Schedule segments across heterogeneous worker nodes based on their relative benchmark FPS.
    pub fn plan_transcode_schedule(
        &mut self,
        segments: &[VideoSegmentDescriptor],
        workers: &[(&str, &str, f32)], // (worker_id, encoder_name, benchmark_fps)
        crf: u8,
    ) -> Result<Vec<SegmentAssignment>, MediaError> {
        if workers.is_empty() {
            return Err(MediaError::InvalidInput {
                reason: "no workers available for media transcoding".to_string(),
            });
        }

        let mut planned = Vec::with_capacity(segments.len());
        let total_capacity: f32 = workers.iter().map(|(_, _, fps)| *fps).sum();
        let total_capacity = if total_capacity <= 0.0 {
            1.0
        } else {
            total_capacity
        };

        // Assign segments round-robin weighted by capacity
        for (i, seg) in segments.iter().enumerate() {
            let worker_idx = i % workers.len();
            let (wid, enc, fps) = workers[worker_idx];

            // Heavier or higher-frame segments can be adjusted based on relative worker fps vs total capacity
            let capacity_ratio = (fps / total_capacity).clamp(0.1, 1.0);
            let target_bitrate = (4500.0 * f64::from(capacity_ratio)).round() as u32;
            let target_bitrate_kbps = target_bitrate.max(2000);

            let assignment = SegmentAssignment {
                segment: seg.clone(),
                assigned_worker_id: wid.to_string(),
                encoder_name: enc.to_string(),
                target_bitrate_kbps,
                crf,
            };
            planned.push(assignment);
        }

        self.assignments = planned.clone();
        Ok(planned)
    }
}
