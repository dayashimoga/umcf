//! Signed discovery beacons and mDNS/DNS-SD TXT record framing for UMCF LAN discovery.
//! Strictly enforces `#![forbid(unsafe_code)]` and zero-placeholder invariants.

use std::net::SocketAddr;

use ed25519_dalek::{Signature, VerifyingKey};
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use umcf_core::{NodeId, NodeRole};
use umcf_identity::NodeKeypair;
use umcf_registry::HlcTimestamp;

use crate::error::NetError;

/// Four-byte magic header for UMCF LAN discovery beacons.
pub const BEACON_MAGIC: [u8; 4] = *b"UMCF";

/// Current discovery beacon wire protocol version.
pub const BEACON_VERSION: u8 = 1;

/// Cryptographic domain separation prefix for LAN discovery beacons.
pub const BEACON_SIGNING_DOMAIN: &[u8] = b"UMCF-LAN-BEACON-v1:";

/// Standard mDNS service type for UMCF mesh peers.
pub const MDNS_SERVICE_TYPE: &str = "_umcf-mesh._udp.local.";

/// Maximum allowed UDP discovery beacon size (bytes) to prevent MTU fragmentation.
pub const MAX_BEACON_PAYLOAD_BYTES: usize = 1400;

/// Parameters for constructing and signing a `SignedDiscoveryBeacon`.
#[derive(Debug, Clone)]
pub struct BeaconAnnouncementParams<'a> {
    pub cluster_id: &'a str,
    pub keypair: &'a NodeKeypair,
    pub hlc_timestamp: HlcTimestamp,
    pub sequence_number: u64,
    pub role: NodeRole,
    pub quic_port: u16,
    pub listen_addrs: SmallVec<[SocketAddr; 4]>,
    pub capability_digest: [u8; 32],
}

/// Signed discovery beacon broadcast/multicast over LAN to announce peer presence.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SignedDiscoveryBeacon {
    /// Magic identifier bytes (`UMCF`).
    pub magic: [u8; 4],
    /// Beacon schema version (`1`).
    pub version: u8,
    /// Target cluster identifier.
    pub cluster_id: String,
    /// Sender's DID string (`did:key:...` or `did:umcf:...`).
    pub sender_did: String,
    /// Raw 32-byte Ed25519 public key.
    pub public_key: [u8; 32],
    /// Monotonic hybrid logical clock timestamp.
    pub hlc_timestamp: HlcTimestamp,
    /// Monotonic sequence number per peer to defeat replay.
    pub sequence_number: u64,
    /// Operational role of the announcing node (Master, Worker, Hybrid).
    pub role: NodeRole,
    /// Reachable QUIC data-plane port on the local host.
    pub quic_port: u16,
    /// Local IPv4 and IPv6 interface addresses.
    pub listen_addrs: SmallVec<[SocketAddr; 4]>,
    /// BLAKE3 digest of hardware capabilities and resource limits.
    pub capability_digest: [u8; 32],
    /// Ed25519 digital signature over the beacon's canonical payload.
    #[serde(with = "signature_serde")]
    pub signature: [u8; 64],
}

impl SignedDiscoveryBeacon {
    /// Creates and cryptographically signs a new `SignedDiscoveryBeacon`.
    #[must_use]
    pub fn sign_new(params: BeaconAnnouncementParams<'_>) -> Self {
        let cluster_id = params.cluster_id.to_string();
        let public_key = params.keypair.ed25519_public_bytes();
        let sender_did = params.keypair.did_key();

        let canonical_bytes = Self::compute_canonical_bytes(
            &cluster_id,
            &sender_did,
            &public_key,
            &params.hlc_timestamp,
            params.sequence_number,
            params.role,
            params.quic_port,
            &params.listen_addrs,
            &params.capability_digest,
        );

        let signature = params.keypair.sign_raw(&canonical_bytes);

        Self {
            magic: BEACON_MAGIC,
            version: BEACON_VERSION,
            cluster_id,
            sender_did,
            public_key,
            hlc_timestamp: params.hlc_timestamp,
            sequence_number: params.sequence_number,
            role: params.role,
            quic_port: params.quic_port,
            listen_addrs: params.listen_addrs,
            capability_digest: params.capability_digest,
            signature,
        }
    }

    /// Verifies the cryptographic signature and format of this beacon.
    ///
    /// # Errors
    /// Returns `NetError::InvalidBeacon` or `NetError::SignatureVerificationFailed` if invalid.
    pub fn verify_signature(&self) -> Result<(), NetError> {
        if self.magic != BEACON_MAGIC {
            return Err(NetError::InvalidBeacon(format!(
                "Invalid magic header: expected {:?}, got {:?}",
                BEACON_MAGIC, self.magic
            )));
        }

        if self.version != BEACON_VERSION {
            return Err(NetError::InvalidBeacon(format!(
                "Unsupported beacon version: expected {BEACON_VERSION}, got {}",
                self.version
            )));
        }

        let canonical_bytes = self.canonical_bytes();

        let vk = VerifyingKey::from_bytes(&self.public_key).map_err(|_| {
            NetError::InvalidBeacon("Invalid Ed25519 public key bytes in beacon".into())
        })?;

        let sig = Signature::from_bytes(&self.signature);
        vk.verify_strict(&canonical_bytes, &sig)
            .map_err(|_| NetError::SignatureVerificationFailed {
                node_id: self.sender_did.clone(),
            })
    }

    /// Returns the canonical sign target bytes for this beacon instance.
    #[must_use]
    pub fn canonical_bytes(&self) -> Vec<u8> {
        Self::compute_canonical_bytes(
            &self.cluster_id,
            &self.sender_did,
            &self.public_key,
            &self.hlc_timestamp,
            self.sequence_number,
            self.role,
            self.quic_port,
            &self.listen_addrs,
            &self.capability_digest,
        )
    }

    /// Derives the canonical UMCF `NodeId` from this beacon's public key.
    ///
    /// # Errors
    /// Returns `NetError::InvalidBeacon` if public key cannot form a valid NodeId.
    pub fn node_id(&self) -> Result<NodeId, NetError> {
        NodeId::from_public_key_bytes(&self.public_key)
            .map_err(|e| NetError::InvalidBeacon(format!("Cannot derive NodeId: {e}")))
    }

    /// Serializes this beacon into deterministic bincode wire format.
    ///
    /// # Errors
    /// Returns `NetError::Serialization` if encoding fails or payload exceeds UDP MTU ceiling.
    pub fn to_bytes(&self) -> Result<Vec<u8>, NetError> {
        let bytes = bincode::serialize(self)
            .map_err(|e| NetError::Serialization(e.to_string()))?;

        if bytes.len() > MAX_BEACON_PAYLOAD_BYTES {
            return Err(NetError::Serialization(format!(
                "Beacon payload size {} bytes exceeds maximum {} bytes ceiling",
                bytes.len(),
                MAX_BEACON_PAYLOAD_BYTES
            )));
        }

        Ok(bytes)
    }

    /// Deserializes a beacon from raw wire bytes without verifying cryptographic signature.
    /// Used for fast cluster filtering and pipeline pre-validation before cryptographic verification.
    ///
    /// # Errors
    /// Returns `NetError::InvalidBeacon` or `NetError::Serialization` if bytes are malformed.
    pub fn from_bytes_unverified(bytes: &[u8]) -> Result<Self, NetError> {
        if bytes.len() < 16 {
            return Err(NetError::InvalidBeacon(format!(
                "Beacon packet too small: {} bytes",
                bytes.len()
            )));
        }

        if bytes.len() > MAX_BEACON_PAYLOAD_BYTES {
            return Err(NetError::InvalidBeacon(format!(
                "Beacon packet exceeds MTU limit: {} bytes",
                bytes.len()
            )));
        }

        let beacon: Self = bincode::deserialize(bytes)
            .map_err(|e| NetError::Serialization(e.to_string()))?;

        if beacon.magic != BEACON_MAGIC {
            return Err(NetError::InvalidBeacon(format!(
                "Invalid magic header: expected {:?}, got {:?}",
                BEACON_MAGIC, beacon.magic
            )));
        }

        if beacon.version != BEACON_VERSION {
            return Err(NetError::InvalidBeacon(format!(
                "Unsupported beacon version: expected {BEACON_VERSION}, got {}",
                beacon.version
            )));
        }

        Ok(beacon)
    }

    /// Deserializes and fully cryptographically validates a beacon from raw wire bytes.
    ///
    /// # Errors
    /// Returns `NetError` if bytes are malformed, signature is invalid, or magic mismatches.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, NetError> {
        let beacon = Self::from_bytes_unverified(bytes)?;
        beacon.verify_signature()?;
        Ok(beacon)
    }

    /// Formats the discovery announcement into mDNS / DNS-SD key-value TXT attributes.
    #[must_use]
    pub fn to_txt_records(&self) -> Vec<(String, String)> {
        vec![
            ("v".into(), self.version.to_string()),
            ("cid".into(), self.cluster_id.clone()),
            ("did".into(), self.sender_did.clone()),
            ("role".into(), format!("{:?}", self.role)),
            ("port".into(), self.quic_port.to_string()),
            ("seq".into(), self.sequence_number.to_string()),
            ("cap".into(), hex::encode(self.capability_digest)),
            ("wall".into(), self.hlc_timestamp.wall_ms.to_string()),
        ]
    }

    /// Constructs canonical sign target bytes for deterministic signing.
    #[allow(clippy::too_many_arguments)]
    fn compute_canonical_bytes(
        cluster_id: &str,
        sender_did: &str,
        public_key: &[u8; 32],
        hlc: &HlcTimestamp,
        seq: u64,
        role: NodeRole,
        quic_port: u16,
        addrs: &[SocketAddr],
        cap_digest: &[u8; 32],
    ) -> Vec<u8> {
        let mut out = Vec::with_capacity(256);
        out.extend_from_slice(BEACON_SIGNING_DOMAIN);
        out.extend_from_slice(cluster_id.as_bytes());
        out.push(b':');
        out.extend_from_slice(sender_did.as_bytes());
        out.push(b':');
        out.extend_from_slice(public_key);
        out.push(b':');
        out.extend_from_slice(&hlc.wall_ms.to_le_bytes());
        out.extend_from_slice(&hlc.logical.to_le_bytes());
        out.extend_from_slice(&seq.to_le_bytes());
        out.push(role as u8);
        out.extend_from_slice(&quic_port.to_le_bytes());
        for addr in addrs {
            match addr {
                SocketAddr::V4(v4) => {
                    out.push(4);
                    out.extend_from_slice(&v4.ip().octets());
                    out.extend_from_slice(&v4.port().to_le_bytes());
                }
                SocketAddr::V6(v6) => {
                    out.push(6);
                    out.extend_from_slice(&v6.ip().octets());
                    out.extend_from_slice(&v6.port().to_le_bytes());
                }
            }
        }
        out.extend_from_slice(cap_digest);
        out
    }
}

mod signature_serde {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(sig: &[u8; 64], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&hex::encode(sig))
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<[u8; 64], D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let bytes = hex::decode(&s).map_err(serde::de::Error::custom)?;
        if bytes.len() != 64 {
            return Err(serde::de::Error::custom("expected 64 bytes for signature"));
        }
        let mut out = [0u8; 64];
        out.copy_from_slice(&bytes);
        Ok(out)
    }
}
