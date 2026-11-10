//! Versioned wire protocol and strict Control-Plane vs. Data-Plane isolation for UMCF.

use std::fmt;

/// Current protocol version implemented by this build.
pub const CURRENT_PROTOCOL_VERSION: ProtocolVersion = ProtocolVersion {
    major: 1,
    minor: 0,
    patch: 0,
};

/// Maximum payload size permitted on the Control Plane (64 KiB).
/// Bulk tensors, model shards, media segments, and storage chunks MUST use the Data Plane.
pub const MAX_CONTROL_PLANE_PAYLOAD_BYTES: usize = 64 * 1024;

/// Semantic protocol version header included in every wire envelope.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProtocolVersion {
    pub major: u16,
    pub minor: u16,
    pub patch: u16,
}

impl ProtocolVersion {
    #[must_use]
    pub const fn new(major: u16, minor: u16, patch: u16) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }

    /// Returns true if `self` (local node) can communicate with `peer` without breaking changes.
    #[must_use]
    pub const fn is_compatible_with(&self, peer: &Self) -> bool {
        self.major == peer.major && self.major > 0
    }

    /// Negotiates the highest mutually supported minor/patch version within the same major version.
    pub fn negotiate(&self, peer: &Self) -> Result<Self, ProtocolError> {
        if !self.is_compatible_with(peer) {
            return Err(ProtocolError::IncompatibleVersion {
                local: *self,
                remote: *peer,
            });
        }
        if (self.minor, self.patch) <= (peer.minor, peer.patch) {
            Ok(*self)
        } else {
            Ok(*peer)
        }
    }
}

impl fmt::Display for ProtocolVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// Identifies whether traffic belongs to the Control Plane or the P2P Data Plane.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PlaneKind {
    /// Signaling, identity attestation, enrollment, capability summaries, and scheduling metadata.
    ControlPlane = 1,
    /// Direct P2P QUIC streams for bulk AI activations/weights, GPU tiles, media, and storage chunks.
    DataPlane = 2,
}

/// Wire message discriminator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MessageKind {
    // Control-plane messages
    Hello = 1,
    AttestIdentity = 2,
    EnrollRequest = 3,
    EnrollGrant = 4,
    Heartbeat = 5,
    CapabilityReport = 6,
    ScheduleTask = 7,
    TaskStatus = 8,
    MeteringReceipt = 9,

    // Data-plane bulk messages
    TensorShardFrame = 100,
    InferenceActivationFrame = 101,
    RenderTileFrame = 102,
    MediaSegmentFrame = 103,
    RemoteStreamVideoFrame = 104,
    RemoteStreamInputFrame = 105,
    StorageChunkFrame = 106,
}

impl MessageKind {
    /// Returns the required plane for this message type.
    #[must_use]
    pub const fn required_plane(self) -> PlaneKind {
        match self {
            Self::Hello
            | Self::AttestIdentity
            | Self::EnrollRequest
            | Self::EnrollGrant
            | Self::Heartbeat
            | Self::CapabilityReport
            | Self::ScheduleTask
            | Self::TaskStatus
            | Self::MeteringReceipt => PlaneKind::ControlPlane,

            Self::TensorShardFrame
            | Self::InferenceActivationFrame
            | Self::RenderTileFrame
            | Self::MediaSegmentFrame
            | Self::RemoteStreamVideoFrame
            | Self::RemoteStreamInputFrame
            | Self::StorageChunkFrame => PlaneKind::DataPlane,
        }
    }

    pub fn from_u16(code: u16) -> Result<Self, ProtocolError> {
        match code {
            1 => Ok(Self::Hello),
            2 => Ok(Self::AttestIdentity),
            3 => Ok(Self::EnrollRequest),
            4 => Ok(Self::EnrollGrant),
            5 => Ok(Self::Heartbeat),
            6 => Ok(Self::CapabilityReport),
            7 => Ok(Self::ScheduleTask),
            8 => Ok(Self::TaskStatus),
            9 => Ok(Self::MeteringReceipt),
            100 => Ok(Self::TensorShardFrame),
            101 => Ok(Self::InferenceActivationFrame),
            102 => Ok(Self::RenderTileFrame),
            103 => Ok(Self::MediaSegmentFrame),
            104 => Ok(Self::RemoteStreamVideoFrame),
            105 => Ok(Self::RemoteStreamInputFrame),
            106 => Ok(Self::StorageChunkFrame),
            other => Err(ProtocolError::UnknownMessageKind(other)),
        }
    }
}

/// Deterministic wire envelope for all UMCF messages.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WireEnvelope {
    pub version: ProtocolVersion,
    pub plane: PlaneKind,
    pub kind: MessageKind,
    pub sequence_id: u64,
    pub sender_node_id: String,
    pub payload: Vec<u8>,
    pub checksum_fnv64: u64,
}

impl WireEnvelope {
    /// Constructs and validates a new `WireEnvelope`, enforcing plane isolation rules.
    pub fn new(
        plane: PlaneKind,
        kind: MessageKind,
        sequence_id: u64,
        sender_node_id: impl Into<String>,
        payload: Vec<u8>,
    ) -> Result<Self, ProtocolError> {
        let sender = sender_node_id.into();
        if sender.trim().is_empty() {
            return Err(ProtocolError::EmptySenderNodeId);
        }
        let expected_plane = kind.required_plane();
        if plane != expected_plane {
            return Err(ProtocolError::PlaneIsolationViolation {
                kind,
                attempted_plane: plane,
                required_plane: expected_plane,
            });
        }
        if plane == PlaneKind::ControlPlane && payload.len() > MAX_CONTROL_PLANE_PAYLOAD_BYTES {
            return Err(ProtocolError::ControlPlanePayloadTooLarge {
                size: payload.len(),
                max_allowed: MAX_CONTROL_PLANE_PAYLOAD_BYTES,
            });
        }

        let checksum_fnv64 = compute_fnv1a_64(&payload);
        Ok(Self {
            version: CURRENT_PROTOCOL_VERSION,
            plane,
            kind,
            sequence_id,
            sender_node_id: sender,
            payload,
            checksum_fnv64,
        })
    }

    /// Verifies envelope integrity, version compatibility, and plane isolation invariants.
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
        let actual_checksum = compute_fnv1a_64(&self.payload);
        if actual_checksum != self.checksum_fnv64 {
            return Err(ProtocolError::ChecksumMismatch {
                expected: self.checksum_fnv64,
                actual: actual_checksum,
            });
        }
        Ok(())
    }

    /// Serializes the envelope into a deterministic binary frame.
    #[must_use]
    pub fn to_bytes(&self) -> Vec<u8> {
        let sender_bytes = self.sender_node_id.as_bytes();
        let sender_len = u16::try_from(sender_bytes.len()).unwrap_or(u16::MAX);
        let payload_len = u32::try_from(self.payload.len()).unwrap_or(u32::MAX);

        let mut buf = Vec::with_capacity(32 + sender_bytes.len() + self.payload.len());
        buf.extend_from_slice(b"UMCF");
        buf.extend_from_slice(&self.version.major.to_be_bytes());
        buf.extend_from_slice(&self.version.minor.to_be_bytes());
        buf.extend_from_slice(&self.version.patch.to_be_bytes());
        buf.push(self.plane as u8);
        buf.extend_from_slice(&(self.kind as u16).to_be_bytes());
        buf.extend_from_slice(&self.sequence_id.to_be_bytes());
        buf.extend_from_slice(&self.checksum_fnv64.to_be_bytes());
        buf.extend_from_slice(&sender_len.to_be_bytes());
        buf.extend_from_slice(&sender_bytes[..usize::from(sender_len)]);
        buf.extend_from_slice(&payload_len.to_be_bytes());
        buf.extend_from_slice(&self.payload[..payload_len as usize]);
        buf
    }

    /// Deserializes and validates a `WireEnvelope` from a binary frame.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ProtocolError> {
        if bytes.len() < 31 || &bytes[0..4] != b"UMCF" {
            return Err(ProtocolError::MalformedFrame("invalid header or length"));
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
        let sequence_id = u64::from_be_bytes([
            bytes[13], bytes[14], bytes[15], bytes[16], bytes[17], bytes[18], bytes[19], bytes[20],
        ]);
        let checksum_fnv64 = u64::from_be_bytes([
            bytes[21], bytes[22], bytes[23], bytes[24], bytes[25], bytes[26], bytes[27], bytes[28],
        ]);
        let sender_len = usize::from(u16::from_be_bytes([bytes[29], bytes[30]]));
        let sender_end = 31 + sender_len;
        if bytes.len() < sender_end + 4 {
            return Err(ProtocolError::MalformedFrame(
                "truncated sender or payload length",
            ));
        }
        let sender_node_id = std::str::from_utf8(&bytes[31..sender_end])
            .map_err(|_| ProtocolError::MalformedFrame("sender_node_id is not valid UTF-8"))?
            .to_owned();
        let payload_len = u32::from_be_bytes([
            bytes[sender_end],
            bytes[sender_end + 1],
            bytes[sender_end + 2],
            bytes[sender_end + 3],
        ]) as usize;
        let payload_start = sender_end + 4;
        if bytes.len() != payload_start + payload_len {
            return Err(ProtocolError::MalformedFrame("payload length mismatch"));
        }
        let payload = bytes[payload_start..].to_vec();
        let env = Self {
            version: ProtocolVersion::new(major, minor, patch),
            plane,
            kind,
            sequence_id,
            sender_node_id,
            payload,
            checksum_fnv64,
        };
        env.verify(&CURRENT_PROTOCOL_VERSION)?;
        Ok(env)
    }
}

/// Deterministic 64-bit FNV-1a checksum for frame integrity verification.
#[must_use]
pub fn compute_fnv1a_64(data: &[u8]) -> u64 {
    const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const FNV_PRIME: u64 = 0x0100_0000_01b3;
    let mut hash = FNV_OFFSET;
    for &byte in data {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

/// Protocol-level errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProtocolError {
    IncompatibleVersion {
        local: ProtocolVersion,
        remote: ProtocolVersion,
    },
    PlaneIsolationViolation {
        kind: MessageKind,
        attempted_plane: PlaneKind,
        required_plane: PlaneKind,
    },
    ControlPlanePayloadTooLarge {
        size: usize,
        max_allowed: usize,
    },
    ChecksumMismatch {
        expected: u64,
        actual: u64,
    },
    EmptySenderNodeId,
    UnknownMessageKind(u16),
    MalformedFrame(&'static str),
}

impl fmt::Display for ProtocolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IncompatibleVersion { local, remote } => {
                write!(f, "incompatible protocol version: local {local} vs remote {remote}")
            }
            Self::PlaneIsolationViolation {
                kind,
                attempted_plane,
                required_plane,
            } => write!(
                f,
                "plane isolation violation for {kind:?}: attempted {attempted_plane:?}, required {required_plane:?}"
            ),
            Self::ControlPlanePayloadTooLarge { size, max_allowed } => write!(
                f,
                "control-plane payload size ({size} bytes) exceeds maximum ({max_allowed} bytes)"
            ),
            Self::ChecksumMismatch { expected, actual } => write!(
                f,
                "frame checksum mismatch: expected {expected:#x}, got {actual:#x}"
            ),
            Self::EmptySenderNodeId => write!(f, "sender_node_id must not be empty"),
            Self::UnknownMessageKind(code) => write!(f, "unknown message kind code: {code}"),
            Self::MalformedFrame(reason) => write!(f, "malformed wire frame: {reason}"),
        }
    }
}

impl std::error::Error for ProtocolError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unit_version_negotiation_and_compatibility() {
        let v1_0 = ProtocolVersion::new(1, 0, 0);
        let v1_2 = ProtocolVersion::new(1, 2, 3);
        let v2_0 = ProtocolVersion::new(2, 0, 0);

        assert!(v1_0.is_compatible_with(&v1_2));
        assert_eq!(v1_0.negotiate(&v1_2), Ok(v1_0));
        assert_eq!(v1_2.negotiate(&v1_0), Ok(v1_0));
        assert!(!v1_0.is_compatible_with(&v2_0));
        assert!(matches!(
            v1_0.negotiate(&v2_0),
            Err(ProtocolError::IncompatibleVersion { .. })
        ));
        assert_eq!(v1_2.to_string(), "1.2.3");
    }

    #[test]
    fn unit_plane_isolation_rejects_bulk_data_on_control_plane() {
        let err = WireEnvelope::new(
            PlaneKind::ControlPlane,
            MessageKind::TensorShardFrame,
            1,
            "node-alpha",
            vec![1, 2, 3, 4],
        );
        assert!(matches!(
            err,
            Err(ProtocolError::PlaneIsolationViolation {
                kind: MessageKind::TensorShardFrame,
                attempted_plane: PlaneKind::ControlPlane,
                required_plane: PlaneKind::DataPlane,
            })
        ));
    }

    #[test]
    fn unit_control_plane_size_cap_enforced() {
        let oversized = vec![0u8; MAX_CONTROL_PLANE_PAYLOAD_BYTES + 1];
        let err = WireEnvelope::new(
            PlaneKind::ControlPlane,
            MessageKind::CapabilityReport,
            2,
            "node-beta",
            oversized,
        );
        assert!(matches!(
            err,
            Err(ProtocolError::ControlPlanePayloadTooLarge { .. })
        ));
    }

    #[test]
    fn unit_wire_envelope_binary_roundtrip_and_tamper_detection() {
        let env = WireEnvelope::new(
            PlaneKind::DataPlane,
            MessageKind::InferenceActivationFrame,
            42,
            "worker-01",
            b"activation-tensor-block".to_vec(),
        )
        .expect("valid data plane envelope");

        let bytes = env.to_bytes();
        let decoded = WireEnvelope::from_bytes(&bytes).expect("roundtrip decode");
        assert_eq!(env, decoded);

        // Tamper with payload byte and confirm checksum failure
        let mut corrupted = bytes.clone();
        let last_idx = corrupted.len() - 1;
        corrupted[last_idx] ^= 0xFF;
        assert!(matches!(
            WireEnvelope::from_bytes(&corrupted),
            Err(ProtocolError::ChecksumMismatch { .. })
        ));
    }

    #[test]
    fn unit_all_message_kinds_roundtrip_and_error_cases() {
        let codes = [1, 2, 3, 4, 5, 6, 7, 8, 9, 100, 101, 102, 103, 104, 105, 106];
        for c in codes {
            let kind = MessageKind::from_u16(c).expect("known code");
            assert_eq!(kind as u16, c);
        }
        assert!(matches!(
            MessageKind::from_u16(999),
            Err(ProtocolError::UnknownMessageKind(999))
        ));
        assert!(matches!(
            WireEnvelope::new(
                PlaneKind::ControlPlane,
                MessageKind::Hello,
                1,
                "   ",
                vec![]
            ),
            Err(ProtocolError::EmptySenderNodeId)
        ));
        assert!(matches!(
            WireEnvelope::from_bytes(b"SHORT"),
            Err(ProtocolError::MalformedFrame(_))
        ));
    }
}
