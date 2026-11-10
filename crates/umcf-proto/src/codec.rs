//! Async streaming length-delimited framing codec for UMCF wire envelopes.
//! Implements `tokio_util::codec::Decoder` and `Encoder` with O(1) header validation.
//! Adheres strictly to `#![forbid(unsafe_code)]` and zero-placeholder invariants.

use bytes::BytesMut;
use thiserror::Error;
use tokio_util::codec::{Decoder, Encoder};

use crate::envelope::{WireEnvelope, FIXED_HEADER_LEN, MAGIC_HEADER};
use crate::{PlaneKind, ProtocolError};

/// Default maximum frame size for control-plane streams (16 MiB).
pub const DEFAULT_CONTROL_PLANE_MAX_FRAME_BYTES: usize = 16 * 1024 * 1024;

/// Default maximum frame size for data-plane streams (64 MiB).
pub const DEFAULT_DATA_PLANE_MAX_FRAME_BYTES: usize = 64 * 1024 * 1024;

/// Errors arising from frame decoding or encoding.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum CodecError {
    #[error("invalid wire magic header: expected 'UMCF' (0x554D4346)")]
    InvalidMagicHeader,

    #[error("frame size ({size} bytes) exceeds maximum configured limit ({max_allowed} bytes)")]
    FrameTooLarge { size: usize, max_allowed: usize },

    #[error("protocol error: {0}")]
    Protocol(#[from] ProtocolError),

    #[error("I/O encoding or decoding failure: {0}")]
    Io(String),
}

impl From<std::io::Error> for CodecError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err.to_string())
    }
}

/// Asynchronous framing codec for length-delimited UMCF envelopes.
#[derive(Debug, Clone)]
pub struct UmcfFrameCodec {
    max_frame_bytes: usize,
    plane: PlaneKind,
}

impl UmcfFrameCodec {
    /// Codec tailored for Control-Plane streams (default limit 16 MiB).
    #[must_use]
    pub fn new_control_plane() -> Self {
        Self {
            max_frame_bytes: DEFAULT_CONTROL_PLANE_MAX_FRAME_BYTES,
            plane: PlaneKind::ControlPlane,
        }
    }

    /// Codec tailored for Data-Plane bulk streams (default limit 64 MiB).
    #[must_use]
    pub fn new_data_plane() -> Self {
        Self {
            max_frame_bytes: DEFAULT_DATA_PLANE_MAX_FRAME_BYTES,
            plane: PlaneKind::DataPlane,
        }
    }

    /// Configures codec with explicit maximum frame size and target plane.
    #[must_use]
    pub fn with_max_frame_bytes(max_frame_bytes: usize, plane: PlaneKind) -> Self {
        Self {
            max_frame_bytes,
            plane,
        }
    }

    #[must_use]
    pub const fn max_frame_bytes(&self) -> usize {
        self.max_frame_bytes
    }

    #[must_use]
    pub const fn plane(&self) -> PlaneKind {
        self.plane
    }
}

impl Default for UmcfFrameCodec {
    fn default() -> Self {
        Self::new_control_plane()
    }
}

impl Decoder for UmcfFrameCodec {
    type Item = WireEnvelope;
    type Error = CodecError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if src.len() < 4 {
            return Ok(None);
        }

        // O(1) Magic header check: reject invalid frames immediately
        if src[0..4] != MAGIC_HEADER {
            return Err(CodecError::InvalidMagicHeader);
        }

        // Need at least fixed header (92 bytes) to parse lengths
        if src.len() < FIXED_HEADER_LEN {
            return Ok(None);
        }

        let sender_len = usize::from(u16::from_be_bytes([src[90], src[91]]));
        let sender_end = FIXED_HEADER_LEN + sender_len;

        // Need sender and payload length prefix (4 bytes)
        if src.len() < sender_end + 4 {
            return Ok(None);
        }

        let payload_len = u32::from_be_bytes([
            src[sender_end],
            src[sender_end + 1],
            src[sender_end + 2],
            src[sender_end + 3],
        ]) as usize;

        // Total required bytes = sender_end + 4 + payload_len + 64 (signature)
        let total_frame_len = sender_end + 4 + payload_len + 64;

        // O(1) Header inspection: Check frame size against limit BEFORE allocating
        if total_frame_len > self.max_frame_bytes {
            return Err(CodecError::FrameTooLarge {
                size: total_frame_len,
                max_allowed: self.max_frame_bytes,
            });
        }

        // If the complete frame is not yet in the buffer, reserve and return Ok(None)
        if src.len() < total_frame_len {
            src.reserve(total_frame_len - src.len());
            return Ok(None);
        }

        // Slice exact frame without excessive allocation
        let frame_bytes = src.split_to(total_frame_len);
        let envelope = WireEnvelope::from_bytes(&frame_bytes)?;
        Ok(Some(envelope))
    }
}

impl Encoder<WireEnvelope> for UmcfFrameCodec {
    type Error = CodecError;

    fn encode(&mut self, item: WireEnvelope, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let serialized = item.to_bytes();
        if serialized.len() > self.max_frame_bytes {
            return Err(CodecError::FrameTooLarge {
                size: serialized.len(),
                max_allowed: self.max_frame_bytes,
            });
        }
        dst.extend_from_slice(&serialized);
        Ok(())
    }
}
