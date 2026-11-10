//! WireEnvelope framing, magic-byte verification, and message metadata.
//! Adheres strictly to `#![forbid(unsafe_code)]` and zero-placeholder invariants.

use std::fmt;
use std::time::SystemTime;

use crate::{
    compute_fnv1a_64, MessageKind, PlaneKind, ProtocolError, ProtocolVersion,
    CURRENT_PROTOCOL_VERSION, MAX_CONTROL_PLANE_PAYLOAD_BYTES,
};

/// 4-byte magic identifier for UMCF wire frames ("UMCF" = 0x554D4346).
pub const MAGIC_HEADER: [u8; 4] = *b"UMCF";
pub const MAGIC_U32: u32 = 0x554d_4346;

/// Fixed-size header length before variable-length sender DID and payload:
/// 4 (magic) + 6 (version) + 1 (plane) + 2 (kind) + 1 (flags) + 16 (msg_id)
/// + 8 (seq_id) + 8 (phys_time) + 4 (logical_ctr) + 32 (blake3) + 8 (fnv64) + 2 (sender_len) = 92 bytes.
pub const FIXED_HEADER_LEN: usize = 92;

/// Minimum complete frame length:
/// 92 (fixed header) + 0 (min sender) + 4 (payload_len) + 0 (min payload) + 64 (signature) = 160 bytes.
pub const MIN_FRAME_LEN: usize = FIXED_HEADER_LEN + 4 + 64;

/// Maximum payload size permitted on the Data Plane (64 MiB).
pub const MAX_DATA_PLANE_PAYLOAD_BYTES: usize = 64 * 1024 * 1024;

/// Wire message flags bitfield.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct MessageFlags(pub u8);

impl MessageFlags {
    pub const NONE: Self = Self(0);
    pub const COMPRESSED_ZSTD: Self = Self(1 << 0);
    pub const ENCRYPTED_PAYLOAD: Self = Self(1 << 1);
    pub const REQUIRES_ACK: Self = Self(1 << 2);

    #[must_use]
    pub const fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    #[must_use]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    #[must_use]
    pub const fn bits(self) -> u8 {
        self.0
    }

    #[must_use]
    pub const fn from_bits_truncate(bits: u8) -> Self {
        Self(bits)
    }
}

/// 128-bit unique message identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MessageId(pub u128);

impl MessageId {
    #[must_use]
    pub const fn new(id: u128) -> Self {
        Self(id)
    }

    #[must_use]
    pub const fn as_u128(self) -> u128 {
        self.0
    }

    #[must_use]
    pub fn from_parts(hi: u64, lo: u64) -> Self {
        Self(((hi as u128) << 64) | (lo as u128))
    }
}

impl fmt::Display for MessageId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:032x}", self.0)
    }
}

/// Hybrid Logical Clock (HLC) combining physical millisecond timestamp with logical counter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TimestampHlc {
    pub physical_time_ms: u64,
    pub logical_counter: u32,
}

impl TimestampHlc {
    #[must_use]
    pub const fn new(physical_time_ms: u64, logical_counter: u32) -> Self {
        Self {
            physical_time_ms,
            logical_counter,
        }
    }

    #[must_use]
    pub fn now() -> Self {
        let ms = match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
            Ok(d) => d.as_millis() as u64,
            Err(_) => 0,
        };
        Self {
            physical_time_ms: ms,
            logical_counter: 0,
        }
    }

    #[must_use]
    pub const fn increment_logical(&self) -> Self {
        Self {
            physical_time_ms: self.physical_time_ms,
            logical_counter: self.logical_counter.saturating_add(1),
        }
    }
}

impl fmt::Display for TimestampHlc {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.physical_time_ms, self.logical_counter)
    }
}

/// Sender Decentralized/Device Identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SenderDid(pub String);

impl SenderDid {
    pub fn new(did: impl Into<String>) -> Result<Self, ProtocolError> {
        let s = did.into();
        if s.trim().is_empty() {
            return Err(ProtocolError::EmptySenderNodeId);
        }
        Ok(Self(s))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for SenderDid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for SenderDid {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl From<String> for SenderDid {
    fn from(s: String) -> Self {
        Self(s)
    }
}

/// Canonical wire envelope for all UMCF messages.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WireEnvelope {
    pub version: ProtocolVersion,
    pub plane: PlaneKind,
    pub kind: MessageKind,
    pub flags: MessageFlags,
    pub message_id: MessageId,
    pub sequence_id: u64,
    pub timestamp_hlc: TimestampHlc,
    pub sender_node_id: String,
    pub sender_did: SenderDid,
    pub payload: Vec<u8>,
    pub checksum_blake3: [u8; 32],
    pub checksum_fnv64: u64,
    pub signature: [u8; 64],
}

impl WireEnvelope {
    /// Constructs a basic `WireEnvelope` matching backward-compatible API.
    pub fn new(
        plane: PlaneKind,
        kind: MessageKind,
        sequence_id: u64,
        sender_node_id: impl Into<String>,
        payload: Vec<u8>,
    ) -> Result<Self, ProtocolError> {
        let sender = sender_node_id.into();
        Self::builder(plane, kind, sender)
            .sequence_id(sequence_id)
            .payload(payload)
            .build()
    }

    /// Creates a fluent builder for `WireEnvelope`.
    pub fn builder(
        plane: PlaneKind,
        kind: MessageKind,
        sender_node_id: impl Into<String>,
    ) -> WireEnvelopeBuilder {
        WireEnvelopeBuilder::new(plane, kind, sender_node_id.into())
    }

    /// Validates envelope constraints, plane separation rules, and checksums.
    pub fn verify(&self, local_version: &ProtocolVersion) -> Result<(), ProtocolError> {
        let _ = local_version.negotiate(&self.version)?;
        if self.sender_node_id.trim().is_empty() {
            return Err(ProtocolError::EmptySenderNodeId);
        }
        let expected_plane = self.kind.required_plane();
        if self.plane != expected_plane {
            return Err(ProtocolError::PlaneIsolationViolation {
                kind: self.kind,
                attempted_plane: self.plane,
                required_plane: expected_plane,
            });
        }
        if self.plane == PlaneKind::ControlPlane
            && self.payload.len() > MAX_CONTROL_PLANE_PAYLOAD_BYTES
        {
            return Err(ProtocolError::ControlPlanePayloadTooLarge {
                size: self.payload.len(),
                max_allowed: MAX_CONTROL_PLANE_PAYLOAD_BYTES,
            });
        }
        let actual_fnv = compute_fnv1a_64(&self.payload);
        if actual_fnv != self.checksum_fnv64 {
            return Err(ProtocolError::ChecksumMismatch {
                expected: self.checksum_fnv64,
                actual: actual_fnv,
            });
        }
        let actual_blake3 = *blake3::hash(&self.payload).as_bytes();
        if actual_blake3 != self.checksum_blake3 {
            return Err(ProtocolError::ChecksumMismatch {
                expected: self.checksum_fnv64,
                actual: compute_fnv1a_64(&actual_blake3),
            });
        }
        Ok(())
    }

    /// Serializes envelope into a contiguous binary byte buffer without unnecessary allocations.
    #[must_use]
    pub fn to_bytes(&self) -> Vec<u8> {
        let compressed_storage;
        let payload_slice: &[u8] = if self.flags.contains(MessageFlags::COMPRESSED_ZSTD) {
            compressed_storage =
                zstd::encode_all(&self.payload[..], 3).unwrap_or_else(|_| self.payload.clone());
            &compressed_storage[..]
        } else {
            &self.payload[..]
        };

        let sender_bytes = self.sender_node_id.as_bytes();
        let sender_len = u16::try_from(sender_bytes.len()).unwrap_or(u16::MAX);
        let payload_len = u32::try_from(payload_slice.len()).unwrap_or(u32::MAX);

        let total_capacity =
            FIXED_HEADER_LEN + usize::from(sender_len) + 4 + payload_slice.len() + 64;
        let mut buf = Vec::with_capacity(total_capacity);

        // Fixed header (92 bytes)
        buf.extend_from_slice(&MAGIC_HEADER);
        buf.extend_from_slice(&self.version.major.to_be_bytes());
        buf.extend_from_slice(&self.version.minor.to_be_bytes());
        buf.extend_from_slice(&self.version.patch.to_be_bytes());
        buf.push(self.plane as u8);
        buf.extend_from_slice(&(self.kind as u16).to_be_bytes());
        buf.push(self.flags.bits());
        buf.extend_from_slice(&self.message_id.0.to_be_bytes());
        buf.extend_from_slice(&self.sequence_id.to_be_bytes());
        buf.extend_from_slice(&self.timestamp_hlc.physical_time_ms.to_be_bytes());
        buf.extend_from_slice(&self.timestamp_hlc.logical_counter.to_be_bytes());
        buf.extend_from_slice(&self.checksum_blake3);
        buf.extend_from_slice(&self.checksum_fnv64.to_be_bytes());
        buf.extend_from_slice(&sender_len.to_be_bytes());

        // Variable sender DID
        buf.extend_from_slice(&sender_bytes[..usize::from(sender_len)]);

        // Variable payload
        buf.extend_from_slice(&payload_len.to_be_bytes());
        buf.extend_from_slice(payload_slice);

        // Ed25519 signature footer slot (64 bytes)
        buf.extend_from_slice(&self.signature);

        buf
    }

    /// Deserializes and validates a `WireEnvelope` from a binary byte buffer with O(1) checks.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ProtocolError> {
        if bytes.len() < MIN_FRAME_LEN {
            return Err(ProtocolError::MalformedFrame(
                "frame shorter than minimum header length",
            ));
        }

        // O(1) Magic header check
        if bytes[0..4] != MAGIC_HEADER {
            return Err(ProtocolError::InvalidMagicHeader);
        }

        let major = u16::from_be_bytes([bytes[4], bytes[5]]);
        let minor = u16::from_be_bytes([bytes[6], bytes[7]]);
        let patch = u16::from_be_bytes([bytes[8], bytes[9]]);

        let plane = match bytes[10] {
            1 => PlaneKind::ControlPlane,
            2 => PlaneKind::DataPlane,
            _ => return Err(ProtocolError::MalformedFrame("invalid plane byte")),
        };

        let kind_code = u16::from_be_bytes([bytes[11], bytes[12]]);
        let kind = MessageKind::from_u16(kind_code)?;
        let flags = MessageFlags(bytes[13]);

        let mut msg_id_bytes = [0u8; 16];
        msg_id_bytes.copy_from_slice(&bytes[14..30]);
        let message_id = MessageId(u128::from_be_bytes(msg_id_bytes));

        let sequence_id = u64::from_be_bytes([
            bytes[30], bytes[31], bytes[32], bytes[33], bytes[34], bytes[35], bytes[36], bytes[37],
        ]);

        let phys_ms = u64::from_be_bytes([
            bytes[38], bytes[39], bytes[40], bytes[41], bytes[42], bytes[43], bytes[44], bytes[45],
        ]);
        let logical_ctr = u32::from_be_bytes([bytes[46], bytes[47], bytes[48], bytes[49]]);
        let timestamp_hlc = TimestampHlc::new(phys_ms, logical_ctr);

        let mut checksum_blake3 = [0u8; 32];
        checksum_blake3.copy_from_slice(&bytes[50..82]);

        let checksum_fnv64 = u64::from_be_bytes([
            bytes[82], bytes[83], bytes[84], bytes[85], bytes[86], bytes[87], bytes[88], bytes[89],
        ]);

        let sender_len = usize::from(u16::from_be_bytes([bytes[90], bytes[91]]));
        let sender_end = FIXED_HEADER_LEN + sender_len;
        if bytes.len() < sender_end + 4 + 64 {
            return Err(ProtocolError::MalformedFrame(
                "frame truncated before payload or signature",
            ));
        }

        let sender_node_id = std::str::from_utf8(&bytes[FIXED_HEADER_LEN..sender_end])
            .map_err(|_| ProtocolError::MalformedFrame("sender_node_id is not valid UTF-8"))?
            .to_owned();
        let sender_did = SenderDid(sender_node_id.clone());

        let payload_len = u32::from_be_bytes([
            bytes[sender_end],
            bytes[sender_end + 1],
            bytes[sender_end + 2],
            bytes[sender_end + 3],
        ]) as usize;

        // O(1) Check against max payload bounds before allocation
        let max_allowed = if plane == PlaneKind::ControlPlane {
            MAX_CONTROL_PLANE_PAYLOAD_BYTES
        } else {
            MAX_DATA_PLANE_PAYLOAD_BYTES
        };
        if payload_len > max_allowed {
            return Err(ProtocolError::ControlPlanePayloadTooLarge {
                size: payload_len,
                max_allowed,
            });
        }

        let payload_start = sender_end + 4;
        let payload_end = payload_start + payload_len;
        let expected_total_len = payload_end + 64;

        if bytes.len() != expected_total_len {
            return Err(ProtocolError::MalformedFrame(
                "frame payload length mismatch",
            ));
        }

        let raw_payload = &bytes[payload_start..payload_end];

        let decompressed_payload = if flags.contains(MessageFlags::COMPRESSED_ZSTD) {
            zstd::decode_all(raw_payload)
                .map_err(|_| ProtocolError::MalformedFrame("zstd decompression failed"))?
        } else {
            raw_payload.to_vec()
        };

        // BLAKE3 integrity check
        let actual_blake3 = *blake3::hash(&decompressed_payload).as_bytes();
        if actual_blake3 != checksum_blake3 {
            return Err(ProtocolError::ChecksumMismatch {
                expected: checksum_fnv64,
                actual: compute_fnv1a_64(&actual_blake3),
            });
        }

        let version = ProtocolVersion::new(major, minor, patch);
        let _ = CURRENT_PROTOCOL_VERSION.negotiate(&version)?;

        let mut signature = [0u8; 64];
        signature.copy_from_slice(&bytes[payload_end..payload_end + 64]);

        Ok(Self {
            version,
            plane,
            kind,
            flags,
            message_id,
            sequence_id,
            timestamp_hlc,
            sender_node_id,
            sender_did,
            payload: decompressed_payload,
            checksum_blake3,
            checksum_fnv64,
            signature,
        })
    }
}

/// Fluent builder for constructing `WireEnvelope`.
#[derive(Debug, Clone)]
pub struct WireEnvelopeBuilder {
    version: ProtocolVersion,
    plane: PlaneKind,
    kind: MessageKind,
    flags: MessageFlags,
    message_id: MessageId,
    sequence_id: u64,
    timestamp_hlc: TimestampHlc,
    sender_node_id: String,
    payload: Vec<u8>,
    signature: [u8; 64],
}

impl WireEnvelopeBuilder {
    pub fn new(plane: PlaneKind, kind: MessageKind, sender_node_id: String) -> Self {
        let now = TimestampHlc::now();
        Self {
            version: CURRENT_PROTOCOL_VERSION,
            plane,
            kind,
            flags: MessageFlags::NONE,
            message_id: MessageId::from_parts(now.physical_time_ms, u64::from(now.logical_counter)),
            sequence_id: now.physical_time_ms,
            timestamp_hlc: now,
            sender_node_id,
            payload: Vec::new(),
            signature: [0u8; 64],
        }
    }

    #[must_use]
    pub fn version(mut self, version: ProtocolVersion) -> Self {
        self.version = version;
        self
    }

    #[must_use]
    pub fn flags(mut self, flags: MessageFlags) -> Self {
        self.flags = flags;
        self
    }

    #[must_use]
    pub fn message_id(mut self, message_id: MessageId) -> Self {
        self.message_id = message_id;
        self
    }

    #[must_use]
    pub fn sequence_id(mut self, sequence_id: u64) -> Self {
        self.sequence_id = sequence_id;
        self
    }

    #[must_use]
    pub fn timestamp_hlc(mut self, timestamp_hlc: TimestampHlc) -> Self {
        self.timestamp_hlc = timestamp_hlc;
        self
    }

    #[must_use]
    pub fn payload(mut self, payload: Vec<u8>) -> Self {
        self.payload = payload;
        self
    }

    #[must_use]
    pub fn signature(mut self, signature: [u8; 64]) -> Self {
        self.signature = signature;
        self
    }

    pub fn build(self) -> Result<WireEnvelope, ProtocolError> {
        let sender = self.sender_node_id.trim();
        if sender.is_empty() {
            return Err(ProtocolError::EmptySenderNodeId);
        }
        let expected_plane = self.kind.required_plane();
        if self.plane != expected_plane {
            return Err(ProtocolError::PlaneIsolationViolation {
                kind: self.kind,
                attempted_plane: self.plane,
                required_plane: expected_plane,
            });
        }
        if self.plane == PlaneKind::ControlPlane
            && self.payload.len() > MAX_CONTROL_PLANE_PAYLOAD_BYTES
        {
            return Err(ProtocolError::ControlPlanePayloadTooLarge {
                size: self.payload.len(),
                max_allowed: MAX_CONTROL_PLANE_PAYLOAD_BYTES,
            });
        }

        let checksum_fnv64 = compute_fnv1a_64(&self.payload);
        let checksum_blake3 = *blake3::hash(&self.payload).as_bytes();

        let sender_did = SenderDid(self.sender_node_id.clone());

        Ok(WireEnvelope {
            version: self.version,
            plane: self.plane,
            kind: self.kind,
            flags: self.flags,
            message_id: self.message_id,
            sequence_id: self.sequence_id,
            timestamp_hlc: self.timestamp_hlc,
            sender_node_id: self.sender_node_id,
            sender_did,
            payload: self.payload,
            checksum_blake3,
            checksum_fnv64,
            signature: self.signature,
        })
    }
}
