//! Universal Mobile Compute Fabric (UMCF) Distributed Media & Transcoding Engine.
//!
//! Provides:
//! - Empirical hardware and software video/audio codec probing with ghost driver elimination.
//! - Unified transcoding adapter with structured sandboxing and hardware-to-software fallback.
//! - Sub-50 ms clean context cancellation.

#![forbid(unsafe_code)]

pub mod adapter;
pub mod error;
pub mod prober;
pub mod segmenter;
pub mod stitcher;

pub use adapter::{
    translate_encoder_parameters, MediaCodecAdapter, NormalizedEncoderArgs, TranscodeProgress,
    TranscodeRequest,
};
pub use error::MediaError;
pub use prober::{
    AccelerationBackend, CandidateCodec, CodecCapabilityDescriptor, CodecDirection,
    HardwareCodecProber, MediaCodecId, PixelFormat,
};
pub use segmenter::{
    GopSegmenter, KeyframeInfo, ParallelTranscodeCoordinator, SegmentAssignment,
    UnifiedEncodingProfile, VideoSegmentDescriptor,
};
pub use stitcher::{
    compute_fractional_pts, ExtradataNormalizer, MediaPacket, SampleAccurateStitcher,
    StitchedMediaContainer, TranscodedSegmentOutput,
};
