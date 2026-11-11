//! Universal Mobile Compute Fabric (UMCF) Ultra-Low-Latency Remote Streaming Engine.
//!
//! Provides:
//! - Direct GPU display capture and window surface extraction.
//! - Sub-millisecond BT.709 NV12 and BT.2020 PQ P010 compute color conversion.
//! - Zero-B-frame, zero-lookahead hardware video encoding with single-frame VBV buffer sizing.
//! - Atomic on-demand IDR keyframe injection and periodic gradual decoder refresh (GDR).
//! - 5 ms / 10 ms low-delay Opus audio loopback packetization with Forward Error Correction.

#![forbid(unsafe_code)]

pub mod abr;
pub mod audio_loopback;
pub mod capture;
pub mod error;
pub mod input_client;
pub mod input_host;
pub mod input_protocol;
pub mod low_latency_encoder;
pub mod transport;

pub use abr::{AbrState, DelayGradientAbrController, StreamResolution};
pub use audio_loopback::{AudioPacket, LoopbackAudioCapturer};
pub use capture::{
    CaptureTarget, CapturedFrame, ColorConverter, CursorMetadata, DisplayCapturer,
    HeadlessSyntheticCapturer,
};
pub use error::StreamError;
pub use input_client::{ActiveControllerRumble, InputReturnClient};
pub use input_host::{
    ActiveInputStateTracker, InputInjectionHost, InputSink, MockInputSink, StreamInputPermission,
    WindowBounds,
};
pub use input_protocol::{
    AntiReplayWindow, CursorShapeType, HostFeedbackEvent, InputEvent, InputEventPacket,
    StylusPhase, TouchPhase,
};
pub use low_latency_encoder::{
    EncodedFrameMetadata, LowLatencyEncoderConfig, ZeroLatencyVideoEncoder,
};
pub use transport::{
    AdaptiveFecCodec, FramePacketizer, MediaShardHeader, MediaShardPacket, NackRequest,
    PliRequest, RecoveryDecision, ZeroDelayJitterBuffer, MAX_DATAGRAM_PAYLOAD_BYTES,
};
