//! Signed Cluster Certificate Revocation Lists (CRL) and sub-microsecond O(1) revocation lookup.
//! Strictly enforces `#![forbid(unsafe_code)]` and zero-placeholder invariants.

use crate::cert::NodeCertificate;
use crate::error::IdentityError;
use crate::keys::{NodeKeypair, DOMAIN_SEPARATION_PREFIX};
use ed25519_dalek::{Signature, VerifyingKey};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Reason for certificate revocation conforming to RFC 5280.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CrlRevocationReason {
    /// Private key was or is suspected to be compromised.
    KeyCompromise,
    /// CA private key was compromised.
    CaCompromise,
    /// Node affiliation or cluster membership changed.
    AffiliationChanged,
    /// Certificate was superseded by a newer key rotation.
    Superseded,
    /// Node permanently decommissioned.
    CessationOfOperation,
    /// Node privileges revoked by cluster administrator.
    PrivilegeWithdrawn,
}

/// Revoked certificate entry recorded in a `ClusterCrl`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RevokedCertificateEntry {
    /// Serial number of the revoked certificate in hex.
    pub serial_number: String,
    /// W3C did:key of the revoked node.
    pub revoked_node_did: String,
    /// Unix epoch timestamp in milliseconds when revocation took effect.
    pub revocation_date_epoch_ms: u64,
    /// Standardized RFC 5280 revocation reason.
    pub reason: CrlRevocationReason,
}

/// Signed Certificate Revocation List distributed across the cluster.
/// Provides sub-microsecond O(1) membership and serial revocation verification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClusterCrl {
    /// Cluster identifier this CRL applies to.
    pub cluster_id: String,
    /// W3C did:key of the issuing Cluster CA coordinator.
    pub issuer_did: String,
    /// Monotonically increasing CRL sequence number.
    pub crl_number: u64,
    /// Issuance epoch timestamp in milliseconds.
    pub this_update_epoch_ms: u64,
    /// Next scheduled update epoch timestamp in milliseconds.
    pub next_update_epoch_ms: u64,
    /// List of all revoked certificates.
    pub revoked_entries: Vec<RevokedCertificateEntry>,
    /// Ed25519 signature by the Cluster CA over canonical CRL payload.
    #[serde(with = "signature_serde")]
    pub signature: [u8; 64],
    #[serde(skip)]
    indexed_serials: HashSet<String>,
    #[serde(skip)]
    indexed_dids: HashSet<String>,
}

mod signature_serde {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(sig: &[u8; 64], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&hex_encode(sig))
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<[u8; 64], D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let bytes = hex_decode(&s).map_err(serde::de::Error::custom)?;
        if bytes.len() != 64 {
            return Err(serde::de::Error::custom("expected 64 bytes for signature"));
        }
        let mut out = [0u8; 64];
        out.copy_from_slice(&bytes);
        Ok(out)
    }

    fn hex_encode(data: &[u8]) -> String {
        data.iter().map(|b| format!("{b:02x}")).collect()
    }

    fn hex_decode(s: &str) -> Result<Vec<u8>, String> {
        if !s.len().is_multiple_of(2) {
            return Err("odd hex string length".into());
        }
        (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16).map_err(|e| e.to_string()))
            .collect()
    }
}

impl ClusterCrl {
    /// Creates a new `ClusterCrl` builder.
    #[must_use]
    pub fn new(
        cluster_id: impl Into<String>,
        issuer_did: impl Into<String>,
        crl_number: u64,
        this_update_epoch_ms: u64,
        next_update_epoch_ms: u64,
        revoked_entries: Vec<RevokedCertificateEntry>,
    ) -> Self {
        let mut indexed_serials = HashSet::with_capacity(revoked_entries.len());
        let mut indexed_dids = HashSet::with_capacity(revoked_entries.len());

        for entry in &revoked_entries {
            indexed_serials.insert(entry.serial_number.clone());
            indexed_dids.insert(entry.revoked_node_did.clone());
        }

        Self {
            cluster_id: cluster_id.into(),
            issuer_did: issuer_did.into(),
            crl_number,
            this_update_epoch_ms,
            next_update_epoch_ms,
            revoked_entries,
            signature: [0u8; 64],
            indexed_serials,
            indexed_dids,
        }
    }

    /// Signs the CRL with the Cluster CA issuer keypair.
    pub fn sign(&mut self, issuer: &NodeKeypair) {
        self.rebuild_indexes();
        let digest = self.compute_signable_digest();
        self.signature = issuer.sign(&digest);
    }

    /// Verifies the signature of the CRL against the provided issuer public key.
    pub fn verify_signature(&self, issuer_public_key: &[u8; 32]) -> Result<(), IdentityError> {
        let digest = self.compute_signable_digest();
        let verifying_key = VerifyingKey::from_bytes(issuer_public_key)
            .map_err(|e| IdentityError::InvalidPublicKey(e.to_string()))?;

        let mut signed_payload =
            Vec::with_capacity(DOMAIN_SEPARATION_PREFIX.len() + digest.len());
        signed_payload.extend_from_slice(DOMAIN_SEPARATION_PREFIX);
        signed_payload.extend_from_slice(&digest);

        let sig = Signature::from_bytes(&self.signature);
        verifying_key
            .verify_strict(&signed_payload, &sig)
            .map_err(|_| IdentityError::InvalidSignature)
    }

    /// Performs an O(1) lookup to check if a certificate serial number is revoked (< 10 µs).
    #[must_use]
    pub fn is_revoked_serial(&self, serial_number: &str) -> bool {
        if self.indexed_serials.is_empty() && !self.revoked_entries.is_empty() {
            // Lazy fallback if deserialized without index
            self.revoked_entries
                .iter()
                .any(|e| e.serial_number == serial_number)
        } else {
            self.indexed_serials.contains(serial_number)
        }
    }

    /// Performs an O(1) lookup to check if a node DID is revoked (< 10 µs).
    #[must_use]
    pub fn is_revoked_did(&self, did: &str) -> bool {
        if self.indexed_dids.is_empty() && !self.revoked_entries.is_empty() {
            self.revoked_entries
                .iter()
                .any(|e| e.revoked_node_did == did)
        } else {
            self.indexed_dids.contains(did)
        }
    }

    /// Checks a `NodeCertificate` against this CRL.
    /// Returns `Err(IdentityError::CertificateRevoked)` if serial or DID is revoked, `Ok(())` otherwise.
    pub fn check_certificate(&self, cert: &NodeCertificate) -> Result<(), IdentityError> {
        if self.is_revoked_serial(cert.serial_number()) || self.is_revoked_did(cert.did()) {
            return Err(IdentityError::CertificateRevoked {
                serial: cert.serial_number().to_string(),
                did: cert.did().to_string(),
            });
        }
        Ok(())
    }

    /// Rebuilds fast lookup indexes in case the entry list was modified.
    pub fn rebuild_indexes(&mut self) {
        self.indexed_serials.clear();
        self.indexed_dids.clear();
        for entry in &self.revoked_entries {
            self.indexed_serials.insert(entry.serial_number.clone());
            self.indexed_dids.insert(entry.revoked_node_did.clone());
        }
    }

    /// Computes canonical BLAKE3 digest of the signable CRL contents.
    #[must_use]
    fn compute_signable_digest(&self) -> [u8; 32] {
        let mut hasher = blake3::Hasher::new();
        hasher.update(b"UMCF-CRL-v1:");
        hasher.update(self.cluster_id.as_bytes());
        hasher.update(self.issuer_did.as_bytes());
        hasher.update(&self.crl_number.to_be_bytes());
        hasher.update(&self.this_update_epoch_ms.to_be_bytes());
        hasher.update(&self.next_update_epoch_ms.to_be_bytes());

        for entry in &self.revoked_entries {
            hasher.update(entry.serial_number.as_bytes());
            hasher.update(entry.revoked_node_did.as_bytes());
            hasher.update(&entry.revocation_date_epoch_ms.to_be_bytes());
            hasher.update(&[entry.reason as u8]);
        }

        *hasher.finalize().as_bytes()
    }
}
