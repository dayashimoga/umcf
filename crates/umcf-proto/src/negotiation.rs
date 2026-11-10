//! Semantic version capability handshake and protocol negotiation for UMCF.
//! Adheres strictly to `#![forbid(unsafe_code)]` and zero-placeholder invariants.

use thiserror::Error;

use crate::envelope::SenderDid;
use crate::ProtocolVersion;

/// Version range defining supported bounds [min, max].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct VersionRange {
    pub min: ProtocolVersion,
    pub max: ProtocolVersion,
}

impl VersionRange {
    #[must_use]
    pub const fn new(min: ProtocolVersion, max: ProtocolVersion) -> Self {
        Self { min, max }
    }

    #[must_use]
    pub const fn exact(version: ProtocolVersion) -> Self {
        Self {
            min: version,
            max: version,
        }
    }

    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.min <= self.max && self.min.major == self.max.major
    }
}

/// Handshake initiation offer sent by a connecting peer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HandshakeOffer {
    pub client_min_version: ProtocolVersion,
    pub client_max_version: ProtocolVersion,
    pub supported_capabilities: Vec<String>,
    pub sender_did: SenderDid,
}

impl HandshakeOffer {
    #[must_use]
    pub fn version_range(&self) -> VersionRange {
        VersionRange::new(self.client_min_version, self.client_max_version)
    }
}

/// Handshake acceptance response returned by the listening peer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HandshakeAccept {
    pub negotiated_version: ProtocolVersion,
    pub agreed_capabilities: Vec<String>,
    pub responder_did: SenderDid,
    pub session_id: u128,
}

/// Handshake rejection message indicating why pairing failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HandshakeReject {
    pub reason_code: u16,
    pub reason_message: String,
    pub supported_versions: Vec<ProtocolVersion>,
}

/// Errors occurring during version and capability negotiation.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum NegotiationError {
    #[error("incompatible major version: local {local_major} vs remote {remote_major}")]
    IncompatibleMajorVersion { local_major: u16, remote_major: u16 },

    #[error("no mutually compatible version found in range")]
    NoCompatibleVersion,

    #[error("invalid version range: min ({min}) exceeds max ({max})")]
    InvalidVersionRange {
        min: ProtocolVersion,
        max: ProtocolVersion,
    },

    #[error("missing required capability: '{0}'")]
    MissingRequiredCapability(String),
}

/// Deterministically negotiates the highest mutually supported protocol version.
///
/// Rejects mismatched major versions with `NegotiationError::IncompatibleMajorVersion`.
/// Rejects non-overlapping ranges with `NegotiationError::NoCompatibleVersion`.
pub fn negotiate_version(
    local_range: VersionRange,
    remote_range: VersionRange,
) -> Result<ProtocolVersion, NegotiationError> {
    if local_range.min > local_range.max {
        return Err(NegotiationError::InvalidVersionRange {
            min: local_range.min,
            max: local_range.max,
        });
    }
    if remote_range.min > remote_range.max {
        return Err(NegotiationError::InvalidVersionRange {
            min: remote_range.min,
            max: remote_range.max,
        });
    }

    if local_range.max.major != remote_range.max.major {
        return Err(NegotiationError::IncompatibleMajorVersion {
            local_major: local_range.max.major,
            remote_major: remote_range.max.major,
        });
    }

    let overlap_min = std::cmp::max(local_range.min, remote_range.min);
    let overlap_max = std::cmp::min(local_range.max, remote_range.max);

    if overlap_min > overlap_max {
        return Err(NegotiationError::NoCompatibleVersion);
    }

    Ok(overlap_max)
}
