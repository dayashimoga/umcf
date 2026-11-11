//! Error taxonomy for UMCF Ultra-Low-Latency Remote Streaming Engine.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum StreamError {
    #[error("display capture failed on target {target}: {reason}")]
    CaptureFailed { target: String, reason: String },

    #[error("color conversion failed: {reason}")]
    ColorConversionFailed { reason: String },

    #[error("encoder configuration invalid: {reason}")]
    InvalidEncoderConfig { reason: String },

    #[error("encoder execution failed: {reason}")]
    EncoderFailed { reason: String },

    #[error("audio loopback capture failed: {reason}")]
    AudioCaptureFailed { reason: String },

    #[error("audio packetization error: {reason}")]
    AudioPacketError { reason: String },

    #[error("target resolution {width}x{height} not supported by encoder")]
    UnsupportedResolution { width: u32, height: u32 },

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}
