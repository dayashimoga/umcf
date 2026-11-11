//! X.509 Certificate generation for UMCF nodes and Cluster Certificate Authorities.
//! Strictly enforces `#![forbid(unsafe_code)]` and zero-placeholder invariants.

use crate::error::IdentityError;
use crate::keys::NodeKeypair;
use rcgen::{
    BasicConstraints, CertificateParams, DistinguishedName, DnType, IsCa, KeyPair, KeyUsagePurpose,
    SanType, SerialNumber, PKCS_ED25519,
};
use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use umcf_core::NodeRole;

/// Cluster-level Certificate Authority issuing leaf certificates to authenticated mesh nodes.
pub struct ClusterCa {
    cluster_id: String,
    ca_keypair: KeyPair,
    ca_serial_bytes: [u8; 16],
    ca_not_before: SystemTime,
    ca_not_after: SystemTime,
    cert_der: Vec<u8>,
    cert_pem: String,
}

impl std::fmt::Debug for ClusterCa {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClusterCa")
            .field("cluster_id", &self.cluster_id)
            .field("cert_der_len", &self.cert_der.len())
            .finish_non_exhaustive()
    }
}

/// Signed membership certificate for an enrolled node in the UMCF fabric.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeCertificate {
    /// W3C did:key of the subject node.
    pub did: String,
    /// Identifier of the cluster this node belongs to.
    pub cluster_id: String,
    /// Operating role assigned to the node within the cluster.
    pub role: NodeRole,
    /// Unique certificate serial number.
    pub serial_number: String,
    /// Validity start epoch timestamp in milliseconds.
    pub not_before_epoch_ms: u64,
    /// Validity expiration epoch timestamp in milliseconds.
    pub not_after_epoch_ms: u64,
    /// Canonical DER-encoded X.509 certificate bytes.
    pub cert_der: Vec<u8>,
    /// PEM-formatted X.509 certificate string.
    pub cert_pem: String,
}

/// Wrapper implementing `rcgen::PublicKeyData` for raw Ed25519 public keys.
#[derive(Debug, Clone, Copy)]
pub struct Ed25519PublicKeyData<'a>(pub &'a [u8; 32]);

impl<'a> rcgen::PublicKeyData for Ed25519PublicKeyData<'a> {
    fn der_bytes(&self) -> &[u8] {
        self.0
    }

    fn algorithm(&self) -> &rcgen::SignatureAlgorithm {
        &rcgen::PKCS_ED25519
    }
}

impl ClusterCa {
    /// Generates a new self-signed root Cluster Certificate Authority.
    pub fn generate(cluster_id: impl Into<String>) -> Result<Self, IdentityError> {
        let cluster_id = cluster_id.into();
        let ca_keypair = KeyPair::generate_for(&PKCS_ED25519)
            .map_err(|e| IdentityError::CertificateInvalid(e.to_string()))?;

        let mut params = CertificateParams::new(vec![format!("{}.umcf.local", cluster_id)])
            .map_err(|e| IdentityError::CertificateInvalid(e.to_string()))?;

        params.distinguished_name = DistinguishedName::new();
        params.distinguished_name.push(
            DnType::CommonName,
            format!("UMCF Root Cluster CA - {}", cluster_id),
        );
        params
            .distinguished_name
            .push(DnType::OrganizationName, "Universal Mobile Compute Fabric");

        params.is_ca = IsCa::Ca(BasicConstraints::Constrained(1));
        params.key_usages = vec![KeyUsagePurpose::KeyCertSign, KeyUsagePurpose::CrlSign];

        // 10 year validity for root CA
        let not_before = SystemTime::now();
        let not_after = not_before + Duration::from_secs(3650 * 86400);
        params.not_before = not_before.into();
        params.not_after = not_after.into();

        let mut serial_bytes = [0u8; 16];
        rand_core::RngCore::fill_bytes(&mut rand_core::OsRng, &mut serial_bytes);
        serial_bytes[0] &= 0x7f;
        params.serial_number = Some(SerialNumber::from_slice(&serial_bytes));

        let cert = params
            .self_signed(&ca_keypair)
            .map_err(|e| IdentityError::CertificateInvalid(e.to_string()))?;

        let cert_der = cert.der().to_vec();
        let cert_pem = cert.pem();

        Ok(Self {
            cluster_id,
            ca_keypair,
            ca_serial_bytes: serial_bytes,
            ca_not_before: not_before,
            ca_not_after: not_after,
            cert_der,
            cert_pem,
        })
    }

    /// Returns the cluster identifier this CA governs.
    #[must_use]
    pub fn cluster_id(&self) -> &str {
        &self.cluster_id
    }

    /// Returns the raw DER bytes of the root CA certificate.
    #[must_use]
    pub fn ca_cert_der(&self) -> &[u8] {
        &self.cert_der
    }

    /// Returns the PEM string of the root CA certificate.
    #[must_use]
    pub fn ca_cert_pem(&self) -> &str {
        &self.cert_pem
    }

    /// Issues an X.509 leaf `NodeCertificate` for an authenticated node keypair.
    pub fn issue_node_certificate(
        &self,
        node_keypair: &NodeKeypair,
        role: NodeRole,
        validity_days: u32,
    ) -> Result<NodeCertificate, IdentityError> {
        self.issue_certificate_for_public_key(
            &node_keypair.ed25519_public_bytes(),
            &node_keypair.did_key(),
            role,
            validity_days,
        )
    }

    /// Issues an X.509 leaf `NodeCertificate` directly for a candidate's public key and DID
    /// without requiring possession of the candidate's private key.
    pub fn issue_certificate_for_public_key(
        &self,
        public_key_bytes: &[u8; 32],
        did: &str,
        role: NodeRole,
        validity_days: u32,
    ) -> Result<NodeCertificate, IdentityError> {
        let public_key_data = Ed25519PublicKeyData(public_key_bytes);

        // Generate cryptographically random 16-byte positive serial number
        let mut serial_bytes = [0u8; 16];
        rand_core::RngCore::fill_bytes(&mut rand_core::OsRng, &mut serial_bytes);
        serial_bytes[0] &= 0x7f; // Ensure positive integer in ASN.1 DER
        let serial_number_hex = hex::encode(serial_bytes);

        // Subject Alternative Names: node DID and cluster domain
        let sans = vec![
            SanType::DnsName(
                format!("{}.umcf.local", self.cluster_id)
                    .try_into()
                    .map_err(|e: rcgen::Error| IdentityError::CertificateInvalid(e.to_string()))?,
            ),
            SanType::URI(
                did.to_string()
                    .try_into()
                    .map_err(|e: rcgen::Error| IdentityError::CertificateInvalid(e.to_string()))?,
            ),
        ];

        let mut params = CertificateParams::default();
        params.serial_number = Some(SerialNumber::from_slice(&serial_bytes));
        params.subject_alt_names = sans;
        params.distinguished_name = DistinguishedName::new();
        params
            .distinguished_name
            .push(DnType::CommonName, did.to_string());
        params
            .distinguished_name
            .push(DnType::OrganizationName, "Universal Mobile Compute Fabric");
        params
            .distinguished_name
            .push(DnType::OrganizationalUnitName, format!("{:?}", role));

        params.is_ca = IsCa::ExplicitNoCa;
        params.key_usages = vec![
            KeyUsagePurpose::DigitalSignature,
            KeyUsagePurpose::KeyAgreement,
        ];

        let now = SystemTime::now();
        let not_before = now.checked_sub(Duration::from_secs(60)).unwrap_or(now);
        let not_after = now + Duration::from_secs(u64::from(validity_days) * 86400);
        params.not_before = not_before.into();
        params.not_after = not_after.into();

        let not_before_ms = not_before
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        let not_after_ms = not_after
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        // Reconstruct CA certificate reference with identical parameters to sign the leaf
        let mut ca_params = CertificateParams::new(vec![format!("{}.umcf.local", self.cluster_id)])
            .map_err(|e| IdentityError::CertificateInvalid(e.to_string()))?;
        ca_params.serial_number = Some(SerialNumber::from_slice(&self.ca_serial_bytes));
        ca_params.distinguished_name = DistinguishedName::new();
        ca_params.distinguished_name.push(
            DnType::CommonName,
            format!("UMCF Root Cluster CA - {}", self.cluster_id),
        );
        ca_params
            .distinguished_name
            .push(DnType::OrganizationName, "Universal Mobile Compute Fabric");
        ca_params.is_ca = IsCa::Ca(BasicConstraints::Constrained(1));
        ca_params.key_usages = vec![KeyUsagePurpose::KeyCertSign, KeyUsagePurpose::CrlSign];
        ca_params.not_before = self.ca_not_before.into();
        ca_params.not_after = self.ca_not_after.into();

        let ca_cert = ca_params
            .self_signed(&self.ca_keypair)
            .map_err(|e| IdentityError::CertificateInvalid(e.to_string()))?;

        let leaf_cert = params
            .signed_by(&public_key_data, &ca_cert, &self.ca_keypair)
            .map_err(|e| IdentityError::CertificateInvalid(e.to_string()))?;

        let cert_der = leaf_cert.der().to_vec();
        let cert_pem = leaf_cert.pem();

        Ok(NodeCertificate {
            did: did.to_string(),
            cluster_id: self.cluster_id.clone(),
            role,
            serial_number: serial_number_hex,
            not_before_epoch_ms: not_before_ms,
            not_after_epoch_ms: not_after_ms,
            cert_der,
            cert_pem,
        })
    }
}

impl NodeCertificate {
    /// Checks whether the certificate is expired at the given timestamp.
    #[must_use]
    pub const fn is_expired(&self, current_epoch_ms: u64) -> bool {
        current_epoch_ms > self.not_after_epoch_ms || current_epoch_ms < self.not_before_epoch_ms
    }

    /// Returns the W3C did:key string embedded in this certificate.
    #[must_use]
    pub fn did(&self) -> &str {
        &self.did
    }

    /// Returns the unique certificate serial number.
    #[must_use]
    pub fn serial_number(&self) -> &str {
        &self.serial_number
    }
}

mod hex {
    pub fn encode(data: impl AsRef<[u8]>) -> String {
        data.as_ref().iter().map(|b| format!("{b:02x}")).collect()
    }
}
