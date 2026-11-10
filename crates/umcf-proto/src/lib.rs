//! Versioned wire protocol, plane isolation, and capability negotiation for UMCF.
//! Adheres strictly to `#![forbid(unsafe_code)]` and zero-placeholder invariants.

pub mod codec;
pub mod envelope;
pub mod negotiation;

pub use codec::{
    CodecError, UmcfFrameCodec, DEFAULT_CONTROL_PLANE_MAX_FRAME_BYTES,
    DEFAULT_DATA_PLANE_MAX_FRAME_BYTES,
};
pub use envelope::{
    MessageFlags, MessageId, SenderDid, TimestampHlc, WireEnvelope, WireEnvelopeBuilder,
    FIXED_HEADER_LEN, MAGIC_HEADER, MAGIC_U32, MAX_DATA_PLANE_PAYLOAD_BYTES, MIN_FRAME_LEN,
};
pub use negotiation::{
    negotiate_version, HandshakeAccept, HandshakeOffer, HandshakeReject, NegotiationError,
    VersionRange,
};

use std::fmt;
use thiserror::Error;

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
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ProtocolError {
    #[error("incompatible protocol version: local {local} vs remote {remote}")]
    IncompatibleVersion {
        local: ProtocolVersion,
        remote: ProtocolVersion,
    },

    #[error("plane isolation violation for {kind:?}: attempted {attempted_plane:?}, required {required_plane:?}")]
    PlaneIsolationViolation {
        kind: MessageKind,
        attempted_plane: PlaneKind,
        required_plane: PlaneKind,
    },

    #[error("control-plane payload size ({size} bytes) exceeds maximum ({max_allowed} bytes)")]
    ControlPlanePayloadTooLarge { size: usize, max_allowed: usize },

    #[error("frame checksum mismatch: expected {expected:#x}, got {actual:#x}")]
    ChecksumMismatch { expected: u64, actual: u64 },

    #[error("sender_node_id must not be empty")]
    EmptySenderNodeId,

    #[error("unknown message kind code: {0}")]
    UnknownMessageKind(u16),

    #[error("invalid wire magic header: expected 'UMCF' (0x554D4346)")]
    InvalidMagicHeader,

    #[error("malformed wire frame: {0}")]
    MalformedFrame(&'static str),
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::BytesMut;
    use std::time::Instant;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio_util::codec::{Decoder, Encoder};

    #[test]
    fn unit_version_negotiation_and_compatibility() {
        let v1_0_0 = ProtocolVersion::new(1, 0, 0);
        let v1_2_1 = ProtocolVersion::new(1, 2, 1);
        let v2_0_0 = ProtocolVersion::new(2, 0, 0);

        assert!(v1_0_0.is_compatible_with(&v1_2_1));
        assert!(v1_2_1.is_compatible_with(&v1_0_0));
        assert!(!v1_0_0.is_compatible_with(&v2_0_0));

        let negotiated = v1_2_1.negotiate(&v1_0_0).expect("negotiate same major");
        assert_eq!(negotiated, v1_0_0);

        let err = v1_0_0.negotiate(&v2_0_0);
        assert!(err.is_err());
    }

    #[test]
    fn unit_semantic_version_range_negotiation_matrix() {
        let local_range =
            VersionRange::new(ProtocolVersion::new(1, 0, 0), ProtocolVersion::new(1, 5, 0));
        let remote_range =
            VersionRange::new(ProtocolVersion::new(1, 2, 0), ProtocolVersion::new(1, 8, 0));

        let negotiated =
            negotiate_version(local_range, remote_range).expect("negotiate version range");
        assert_eq!(negotiated, ProtocolVersion::new(1, 5, 0));

        // Mismatched major versions must return IncompatibleMajorVersion
        let v2_range =
            VersionRange::new(ProtocolVersion::new(2, 0, 0), ProtocolVersion::new(2, 1, 0));
        let err = negotiate_version(local_range, v2_range);
        assert!(matches!(
            err,
            Err(NegotiationError::IncompatibleMajorVersion {
                local_major: 1,
                remote_major: 2
            })
        ));

        // Disjoint version ranges in same major
        let low_range =
            VersionRange::new(ProtocolVersion::new(1, 0, 0), ProtocolVersion::new(1, 1, 0));
        let high_range =
            VersionRange::new(ProtocolVersion::new(1, 5, 0), ProtocolVersion::new(1, 6, 0));
        assert!(matches!(
            negotiate_version(low_range, high_range),
            Err(NegotiationError::NoCompatibleVersion)
        ));
    }

    #[test]
    fn unit_wire_envelope_flags_and_metadata() {
        let flags = MessageFlags::NONE
            .union(MessageFlags::COMPRESSED_ZSTD)
            .union(MessageFlags::REQUIRES_ACK);
        assert!(flags.contains(MessageFlags::COMPRESSED_ZSTD));
        assert!(flags.contains(MessageFlags::REQUIRES_ACK));
        assert!(!flags.contains(MessageFlags::ENCRYPTED_PAYLOAD));

        let msg_id = MessageId::new(0x1234_5678_9abc_def0_1122_3344_5566_7788);
        assert_eq!(msg_id.as_u128(), 0x1234_5678_9abc_def0_1122_3344_5566_7788);

        let hlc = TimestampHlc::new(1_700_000_000_000, 42);
        assert_eq!(hlc.physical_time_ms, 1_700_000_000_000);
        assert_eq!(hlc.logical_counter, 42);

        let did = SenderDid::new("did:umcf:node-unit-test-01").expect("valid did");
        assert_eq!(did.as_str(), "did:umcf:node-unit-test-01");
    }

    #[test]
    fn unit_wire_envelope_zstd_compression_toggle() {
        let uncompressed_payload = b"Hello, UMCF! This is repeated payload data. ".repeat(64);
        let env = WireEnvelope::builder(
            PlaneKind::ControlPlane,
            MessageKind::Heartbeat,
            "did:umcf:test-compress-node",
        )
        .flags(MessageFlags::COMPRESSED_ZSTD)
        .payload(uncompressed_payload.clone())
        .build()
        .expect("build envelope");

        let bytes = env.to_bytes();
        let decoded = WireEnvelope::from_bytes(&bytes).expect("decode compressed envelope");
        assert_eq!(decoded.payload, uncompressed_payload);
        assert!(decoded.flags.contains(MessageFlags::COMPRESSED_ZSTD));
    }

    #[test]
    fn unit_wire_envelope_micro_benchmark_latency() {
        // Acceptance criteria: Round-trip serialization/deserialization takes < 2.5 µs for 1 KiB payload
        let payload = vec![0x42u8; 1024]; // 1 KiB
        let env = WireEnvelope::new(
            PlaneKind::ControlPlane,
            MessageKind::Heartbeat,
            1,
            "did:umcf:benchmark-node-01",
            payload,
        )
        .expect("envelope");

        // Warm up
        for _ in 0..100 {
            let b = env.to_bytes();
            let _ = WireEnvelope::from_bytes(&b).expect("warmup");
        }

        let iterations = 10_000;
        let start = Instant::now();
        for _ in 0..iterations {
            let b = env.to_bytes();
            let _ = WireEnvelope::from_bytes(&b).expect("decode");
        }
        let total_duration = start.elapsed();
        let per_op_micros = total_duration.as_secs_f64() * 1_000_000.0 / f64::from(iterations);

        println!(
            "Micro-benchmark: 1 KiB roundtrip took {per_op_micros:.3} µs per op (release target < 2.5 µs)"
        );
        #[cfg(not(debug_assertions))]
        assert!(
            per_op_micros < 2.5,
            "Serialization roundtrip in release must be < 2.5 µs, got {per_op_micros:.3} µs"
        );
        #[cfg(debug_assertions)]
        assert!(
            per_op_micros < 10.0,
            "Serialization roundtrip in debug must be < 10.0 µs, got {per_op_micros:.3} µs"
        );
    }

    #[test]
    fn unit_control_plane_size_cap_enforced() {
        let oversized_payload = vec![0u8; MAX_CONTROL_PLANE_PAYLOAD_BYTES + 1];
        let err = WireEnvelope::new(
            PlaneKind::ControlPlane,
            MessageKind::Heartbeat,
            1,
            "did:umcf:node-01",
            oversized_payload,
        );
        assert!(matches!(
            err,
            Err(ProtocolError::ControlPlanePayloadTooLarge { .. })
        ));
    }

    #[test]
    fn unit_plane_isolation_rejects_bulk_data_on_control_plane() {
        let err = WireEnvelope::new(
            PlaneKind::ControlPlane,
            MessageKind::TensorShardFrame,
            1,
            "did:umcf:node-01",
            vec![1, 2, 3],
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
    fn unit_all_message_kinds_roundtrip_and_error_cases() {
        let kinds = [
            MessageKind::Hello,
            MessageKind::AttestIdentity,
            MessageKind::EnrollRequest,
            MessageKind::EnrollGrant,
            MessageKind::Heartbeat,
            MessageKind::CapabilityReport,
            MessageKind::ScheduleTask,
            MessageKind::TaskStatus,
            MessageKind::MeteringReceipt,
        ];
        for kind in kinds {
            let env = WireEnvelope::new(
                kind.required_plane(),
                kind,
                100,
                "did:umcf:all-kinds-node",
                vec![1, 2, 3, 4],
            )
            .expect("valid env");
            let encoded = env.to_bytes();
            let decoded = WireEnvelope::from_bytes(&encoded).expect("decode");
            assert_eq!(decoded.kind, kind);
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn integration_stream_10000_framed_envelopes_over_duplex_stream() {
        let (mut client_io, mut server_io) = tokio::io::duplex(64 * 1024);
        let message_count = 10_000;

        let writer_handle = tokio::spawn(async move {
            let mut codec = UmcfFrameCodec::new_control_plane();
            let mut buf = BytesMut::with_capacity(32 * 1024);

            for seq in 0..message_count {
                let env = WireEnvelope::new(
                    PlaneKind::ControlPlane,
                    MessageKind::Heartbeat,
                    seq,
                    "did:umcf:stream-node-writer",
                    seq.to_be_bytes().to_vec(),
                )
                .expect("envelope");

                codec.encode(env, &mut buf).expect("encode frame");

                if buf.len() >= 16 * 1024 || seq == message_count - 1 {
                    client_io.write_all(&buf).await.expect("write to duplex");
                    buf.clear();
                }
            }
        });

        let reader_handle = tokio::spawn(async move {
            let mut codec = UmcfFrameCodec::new_control_plane();
            let mut read_buf = BytesMut::with_capacity(64 * 1024);
            let mut chunk = [0u8; 8192];
            let mut received_count = 0u64;

            while received_count < message_count {
                let n = server_io.read(&mut chunk).await.expect("read from duplex");
                if n == 0 {
                    break;
                }
                read_buf.extend_from_slice(&chunk[..n]);

                while let Some(env) = codec.decode(&mut read_buf).expect("decode envelope") {
                    assert_eq!(env.sequence_id, received_count);
                    assert_eq!(env.payload, received_count.to_be_bytes());
                    received_count += 1;
                }
            }
            assert_eq!(received_count, message_count);
        });

        writer_handle.await.expect("writer task");
        reader_handle.await.expect("reader task");
    }

    #[test]
    fn simulation_header_magic_and_length_fault_injection_oom_defense() {
        let valid_env = WireEnvelope::new(
            PlaneKind::ControlPlane,
            MessageKind::Heartbeat,
            1,
            "did:umcf:fault-injection-node",
            vec![1, 2, 3, 4],
        )
        .expect("valid envelope");
        let valid_bytes = valid_env.to_bytes();

        // 1. Invalid magic header test
        let mut corrupted_magic = valid_bytes.clone();
        corrupted_magic[0] = b'X';
        assert!(matches!(
            WireEnvelope::from_bytes(&corrupted_magic),
            Err(ProtocolError::InvalidMagicHeader)
        ));

        // 2. OOM attack length prefix defense: 0xFFFFFFFF payload length
        let mut oom_attack_bytes = valid_bytes.clone();
        let sender_len = usize::from(u16::from_be_bytes([
            oom_attack_bytes[90],
            oom_attack_bytes[91],
        ]));
        let payload_len_offset = FIXED_HEADER_LEN + sender_len;
        oom_attack_bytes[payload_len_offset] = 0xFF;
        oom_attack_bytes[payload_len_offset + 1] = 0xFF;
        oom_attack_bytes[payload_len_offset + 2] = 0xFF;
        oom_attack_bytes[payload_len_offset + 3] = 0xFF;

        let mut codec = UmcfFrameCodec::new_control_plane();
        let mut buf = BytesMut::from(&oom_attack_bytes[..]);
        let err = codec.decode(&mut buf);
        assert!(matches!(err, Err(CodecError::FrameTooLarge { .. })));

        // 3. Checksum tampering detection
        let mut corrupted_payload = valid_bytes.clone();
        let payload_offset = payload_len_offset + 4;
        corrupted_payload[payload_offset] ^= 0xFF;
        assert!(matches!(
            WireEnvelope::from_bytes(&corrupted_payload),
            Err(ProtocolError::ChecksumMismatch { .. })
        ));
    }

    #[test]
    fn simulation_10000_bit_flip_fault_injections_no_panics() {
        let base_env = WireEnvelope::new(
            PlaneKind::ControlPlane,
            MessageKind::Heartbeat,
            42,
            "did:umcf:mutation-node-01",
            vec![10, 20, 30, 40, 50],
        )
        .expect("base envelope");
        let base_bytes = base_env.to_bytes();

        let mut codec = UmcfFrameCodec::new_control_plane();

        // Deterministic pseudo-random seed
        let mut state: u64 = 0x853c_49e6_748f_ea9b;
        let mut lcg = || {
            state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
            state
        };

        for _ in 0..10_000 {
            let mut mutated = base_bytes.clone();
            let flip_count = (lcg() % 5) + 1;
            for _ in 0..flip_count {
                let idx = (lcg() as usize) % mutated.len();
                let bit = 1 << ((lcg() % 8) as u8);
                mutated[idx] ^= bit;
            }

            // Zero panics or unhandled exceptions allowed
            let _ = WireEnvelope::from_bytes(&mutated);
            let mut buf = BytesMut::from(&mutated[..]);
            let _ = codec.decode(&mut buf);
        }
    }
}
