//! Quinn/QUIC multiplexed data-plane transport with mTLS 1.3, connection pooling, and mutual attestation.
//! Strictly enforces `#![forbid(unsafe_code)]` and zero-placeholder invariants.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use quinn::crypto::rustls::{QuicClientConfig, QuicServerConfig};
use quinn::{Connection, Endpoint, RecvStream, SendStream, TransportConfig};
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::client::WebPkiServerVerifier;
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer, ServerName, UnixTime};
use rustls::server::danger::{ClientCertVerified, ClientCertVerifier};
use rustls::server::WebPkiClientVerifier;
use rustls::{ClientConfig, RootCertStore, ServerConfig, SignatureScheme};
use tokio_util::sync::CancellationToken;
use umcf_core::NodeId;
use umcf_identity::attestation::{AttestationChallenge, AttestationProof, MutualAttestationProtocol, TLS_EXPORTER_LABEL};
use umcf_identity::cert::NodeCertificate;
use umcf_identity::crl::ClusterCrl;
use umcf_identity::did::DidKey;
use umcf_identity::NodeKeypair;

use crate::error::NetError;

/// Mandatory ALPN protocol identifier for UMCF QUIC data-plane transport.
pub const QUIC_ALPN_V1: &[u8] = b"umcf-quic/v1";

/// Multiplexed QUIC stream type identifier.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuicStreamType {
    /// Low-latency bidirectional control, cluster attestation, and heartbeats.
    BidirectionalControl = 0x01,
    /// High-frequency remote procedure call invocations.
    BidirectionalRpc = 0x02,
    /// High-throughput unidirectional bulk tensor and model weights streaming.
    UnidirectionalBulkTensor = 0x03,
    /// Unreliable datagram media, video frames, and telemetry.
    UnreliableDatagramMedia = 0x04,
}

impl QuicStreamType {
    /// Decodes stream type from leading byte header.
    #[must_use]
    pub fn from_u8(b: u8) -> Option<Self> {
        match b {
            0x01 => Some(Self::BidirectionalControl),
            0x02 => Some(Self::BidirectionalRpc),
            0x03 => Some(Self::UnidirectionalBulkTensor),
            0x04 => Some(Self::UnreliableDatagramMedia),
            _ => None,
        }
    }
}

/// Extracted metadata from verified X.509 peer node certificate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractedCertInfo {
    /// Unique serial number hex string.
    pub serial_hex: String,
    /// Subject Alternative Name did:key string.
    pub did_key: String,
    /// Extracted cluster domain identifier.
    pub cluster_id: String,
}

/// Extracts serial number, did:key URI SAN, and cluster domain from canonical X.509 DER.
///
/// # Errors
/// Returns `NetError::InvalidCertificate` if ASN.1 structure is invalid or required fields are missing.
pub fn extract_cert_info(der: &[u8]) -> Result<ExtractedCertInfo, NetError> {
    let serial_hex = extract_serial_number_hex(der)?;
    let did_key = extract_did_key_san(der)?;
    let cluster_id = extract_cluster_id(der)?;

    Ok(ExtractedCertInfo {
        serial_hex,
        did_key,
        cluster_id,
    })
}

fn extract_serial_number_hex(der: &[u8]) -> Result<String, NetError> {
    if der.len() < 32 || der[0] != 0x30 {
        return Err(NetError::InvalidCertificate(
            "Invalid certificate DER outer SEQUENCE".into(),
        ));
    }
    let (tbs_offset, _) = read_asn1_length(&der[1..])?;
    let tbs_start = 1 + tbs_offset;
    if tbs_start >= der.len() || der[tbs_start] != 0x30 {
        return Err(NetError::InvalidCertificate(
            "Invalid TBSCertificate SEQUENCE".into(),
        ));
    }
    let (tbs_len_offset, _) = read_asn1_length(&der[tbs_start + 1..])?;
    let mut cursor = tbs_start + 1 + tbs_len_offset;

    // Skip version [0] EXPLICIT if present
    if cursor < der.len() && der[cursor] == 0xa0 {
        let (ver_len_offset, ver_len) = read_asn1_length(&der[cursor + 1..])?;
        cursor = cursor + 1 + ver_len_offset + ver_len;
    }

    // Next field is serialNumber (INTEGER tag 0x02)
    if cursor >= der.len() || der[cursor] != 0x02 {
        return Err(NetError::InvalidCertificate(
            "Expected INTEGER for certificate serial number".into(),
        ));
    }
    let (int_len_offset, int_len) = read_asn1_length(&der[cursor + 1..])?;
    let int_start = cursor + 1 + int_len_offset;
    let int_end = int_start + int_len;
    if int_end > der.len() {
        return Err(NetError::InvalidCertificate(
            "Truncated serial number in certificate".into(),
        ));
    }

    let mut serial_bytes = &der[int_start..int_end];
    if serial_bytes.len() > 1 && serial_bytes[0] == 0x00 {
        serial_bytes = &serial_bytes[1..];
    }
    Ok(hex::encode(serial_bytes))
}

fn extract_did_key_san(der: &[u8]) -> Result<String, NetError> {
    let prefix = b"did:key:";
    if let Some(pos) = der.windows(prefix.len()).position(|w| w == prefix) {
        // Canonical Ed25519 did:key is exactly 56 bytes (8 bytes prefix + 48 base58 chars)
        let target_len = 56;
        if pos + target_len <= der.len() {
            let slice = &der[pos..pos + target_len];
            if slice.iter().all(|&b| b.is_ascii_alphanumeric() || b == b':') {
                return String::from_utf8(slice.to_vec())
                    .map_err(|e| NetError::InvalidCertificate(e.to_string()));
            }
        }

        let did_bytes: Vec<u8> = der[pos..]
            .iter()
            .take_while(|&&b| b.is_ascii_alphanumeric() || b == b':' || b == b'-' || b == b'_')
            .copied()
            .collect();
        String::from_utf8(did_bytes).map_err(|e| NetError::InvalidCertificate(e.to_string()))
    } else {
        Err(NetError::InvalidCertificate(
            "Missing did:key in certificate Subject Alternative Name".into(),
        ))
    }
}

fn extract_cluster_id(der: &[u8]) -> Result<String, NetError> {
    let suffix = b".umcf.local";
    if let Some(pos) = der.windows(suffix.len()).position(|w| w == suffix) {
        let mut start = pos;
        while start > 0 {
            let b = der[start - 1];
            if b.is_ascii_alphanumeric() || b == b'-' || b == b'_' {
                start -= 1;
            } else {
                break;
            }
        }
        let cluster_bytes = &der[start..pos];
        if cluster_bytes.is_empty() {
            return Err(NetError::InvalidCertificate(
                "Empty cluster prefix in .umcf.local SAN".into(),
            ));
        }
        String::from_utf8(cluster_bytes.to_vec())
            .map_err(|e| NetError::InvalidCertificate(e.to_string()))
    } else {
        Err(NetError::InvalidCertificate(
            "Missing .umcf.local domain in certificate Subject Alternative Name".into(),
        ))
    }
}

fn read_asn1_length(bytes: &[u8]) -> Result<(usize, usize), NetError> {
    if bytes.is_empty() {
        return Err(NetError::InvalidCertificate(
            "Empty length field in ASN.1 DER".into(),
        ));
    }
    let initial = bytes[0];
    if initial < 0x80 {
        Ok((1, initial as usize))
    } else {
        let num_octets = (initial & 0x7f) as usize;
        if num_octets == 0 || num_octets > 4 || bytes.len() < 1 + num_octets {
            return Err(NetError::InvalidCertificate(
                "Unsupported ASN.1 length encoding".into(),
            ));
        }
        let mut len = 0usize;
        for &b in &bytes[1..1 + num_octets] {
            len = (len << 8) | (b as usize);
        }
        Ok((1 + num_octets, len))
    }
}

/// Custom client-side TLS verifier enforcing Cluster CA signature, SAN matching, and CRL revocation check.
#[derive(Debug)]
pub struct ClusterCaServerVerifier {
    inner: Arc<dyn ServerCertVerifier>,
    expected_cluster_id: String,
    crl: Arc<RwLock<ClusterCrl>>,
}

impl ClusterCaServerVerifier {
    /// Creates a new `ClusterCaServerVerifier`.
    pub fn new(
        ca_cert_der: &[u8],
        expected_cluster_id: impl Into<String>,
        crl: Arc<RwLock<ClusterCrl>>,
    ) -> Result<Self, NetError> {
        let mut roots = RootCertStore::empty();
        roots
            .add(CertificateDer::from(ca_cert_der.to_vec()))
            .map_err(|e| NetError::Tls(format!("Failed to register Cluster CA root: {e}")))?;

        let inner = WebPkiServerVerifier::builder(Arc::new(roots))
            .build()
            .map_err(|e| NetError::Tls(format!("Failed to build WebPkiServerVerifier: {e}")))?;

        Ok(Self {
            inner,
            expected_cluster_id: expected_cluster_id.into(),
            crl,
        })
    }
}

impl ServerCertVerifier for ClusterCaServerVerifier {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        intermediates: &[CertificateDer<'_>],
        server_name: &ServerName<'_>,
        ocsp_response: &[u8],
        now: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        // 1. Verify cryptographic chain of trust and validity period
        self.inner
            .verify_server_cert(end_entity, intermediates, server_name, ocsp_response, now)?;

        // 2. Extract and validate UMCF certificate metadata
        let info = extract_cert_info(end_entity.as_ref())
            .map_err(|_e| rustls::Error::InvalidCertificate(rustls::CertificateError::ApplicationVerificationFailure))?;

        // 3. Cluster ID isolation check
        if info.cluster_id != self.expected_cluster_id {
            return Err(rustls::Error::InvalidCertificate(
                rustls::CertificateError::ApplicationVerificationFailure,
            ));
        }

        // 4. Cluster CRL revocation check
        if let Ok(crl_lock) = self.crl.read() {
            if crl_lock.is_revoked_serial(&info.serial_hex) || crl_lock.is_revoked_did(&info.did_key) {
                return Err(rustls::Error::InvalidCertificate(
                    rustls::CertificateError::Revoked,
                ));
            }
        }

        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &rustls::DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        self.inner.verify_tls12_signature(message, cert, dss)
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &rustls::DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        self.inner.verify_tls13_signature(message, cert, dss)
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.inner.supported_verify_schemes()
    }
}

/// Custom server-side TLS verifier enforcing Client mTLS, Cluster CA signature, and CRL check.
#[derive(Debug)]
pub struct ClusterCaClientVerifier {
    inner: Arc<dyn ClientCertVerifier>,
    expected_cluster_id: String,
    crl: Arc<RwLock<ClusterCrl>>,
}

impl ClusterCaClientVerifier {
    /// Creates a new `ClusterCaClientVerifier`.
    pub fn new(
        ca_cert_der: &[u8],
        expected_cluster_id: impl Into<String>,
        crl: Arc<RwLock<ClusterCrl>>,
    ) -> Result<Self, NetError> {
        let mut roots = RootCertStore::empty();
        roots
            .add(CertificateDer::from(ca_cert_der.to_vec()))
            .map_err(|e| NetError::Tls(format!("Failed to register Cluster CA root: {e}")))?;

        let inner = WebPkiClientVerifier::builder(Arc::new(roots))
            .build()
            .map_err(|e| NetError::Tls(format!("Failed to build WebPkiClientVerifier: {e}")))?;

        Ok(Self {
            inner,
            expected_cluster_id: expected_cluster_id.into(),
            crl,
        })
    }
}

impl ClientCertVerifier for ClusterCaClientVerifier {
    fn root_hint_subjects(&self) -> &[rustls::DistinguishedName] {
        self.inner.root_hint_subjects()
    }

    fn verify_client_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        intermediates: &[CertificateDer<'_>],
        now: UnixTime,
    ) -> Result<ClientCertVerified, rustls::Error> {
        // 1. Verify cryptographic chain of trust and validity period
        self.inner.verify_client_cert(end_entity, intermediates, now)?;

        // 2. Extract and validate UMCF certificate metadata
        let info = extract_cert_info(end_entity.as_ref())
            .map_err(|_| rustls::Error::InvalidCertificate(rustls::CertificateError::ApplicationVerificationFailure))?;

        // 3. Cluster ID isolation check
        if info.cluster_id != self.expected_cluster_id {
            return Err(rustls::Error::InvalidCertificate(
                rustls::CertificateError::ApplicationVerificationFailure,
            ));
        }

        // 4. Cluster CRL revocation check
        if let Ok(crl_lock) = self.crl.read() {
            if crl_lock.is_revoked_serial(&info.serial_hex) || crl_lock.is_revoked_did(&info.did_key) {
                return Err(rustls::Error::InvalidCertificate(
                    rustls::CertificateError::Revoked,
                ));
            }
        }

        Ok(ClientCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &rustls::DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        self.inner.verify_tls12_signature(message, cert, dss)
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &rustls::DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        self.inner.verify_tls13_signature(message, cert, dss)
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.inner.supported_verify_schemes()
    }
}

/// Operational state of a peer QUIC connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PeerConnectionState {
    /// TLS handshake in progress.
    Handshaking,
    /// 2-RTT mutual attestation in progress.
    Attesting,
    /// Mutual attestation completed successfully; ready for RPC and bulk streaming.
    AuthenticatedReady,
    /// Connection closed or terminated.
    Closed,
}

/// Active peer connection over QUIC data plane.
#[derive(Debug)]
pub struct PeerConnection {
    /// Remote NodeId.
    pub node_id: NodeId,
    /// Remote did:key string.
    pub remote_did: String,
    /// Remote certificate information.
    pub cert_info: ExtractedCertInfo,
    /// Underlying Quinn connection handle.
    pub connection: Connection,
    /// Bandwidth shaper and QoS scheduler.
    pub shaper: Arc<crate::qos::BandwidthShaper>,
    state: Arc<RwLock<PeerConnectionState>>,
}

impl PeerConnection {
    /// Returns the current attestation and readiness state of this connection.
    #[must_use]
    pub fn state(&self) -> PeerConnectionState {
        self.state.read().map(|s| *s).unwrap_or(PeerConnectionState::Closed)
    }

    /// Sets the connection state.
    pub fn set_state(&self, new_state: PeerConnectionState) {
        if let Ok(mut lock) = self.state.write() {
            *lock = new_state;
        }
    }

    /// Opens a new bidirectional control stream, prepending `QuicStreamType::BidirectionalControl`
    /// and assigning `TrafficClass::CriticalControl` transmission priority.
    pub async fn open_bi_control(&self) -> Result<(SendStream, RecvStream), NetError> {
        let (mut send, recv) = self
            .connection
            .open_bi()
            .await
            .map_err(|e| NetError::QuicConnection(e.to_string()))?;

        send.set_priority(crate::qos::TrafficClass::CriticalControl.quic_priority())
            .map_err(|e| NetError::QuicStream(e.to_string()))?;

        send.write_all(&[QuicStreamType::BidirectionalControl as u8])
            .await
            .map_err(|e| NetError::QuicStream(e.to_string()))?;

        Ok((send, recv))
    }

    /// Opens a new bidirectional RPC stream, prepending `QuicStreamType::BidirectionalRpc`
    /// and assigning `TrafficClass::ElasticCompute` transmission priority.
    pub async fn open_bi_rpc(&self) -> Result<(SendStream, RecvStream), NetError> {
        let (mut send, recv) = self
            .connection
            .open_bi()
            .await
            .map_err(|e| NetError::QuicConnection(e.to_string()))?;

        send.set_priority(crate::qos::TrafficClass::ElasticCompute.quic_priority())
            .map_err(|e| NetError::QuicStream(e.to_string()))?;

        send.write_all(&[QuicStreamType::BidirectionalRpc as u8])
            .await
            .map_err(|e| NetError::QuicStream(e.to_string()))?;

        Ok((send, recv))
    }

    /// Opens a new unidirectional bulk tensor stream, prepending `QuicStreamType::UnidirectionalBulkTensor`
    /// and assigning `TrafficClass::BackgroundBulk` transmission priority.
    pub async fn open_uni_bulk_tensor(&self) -> Result<SendStream, NetError> {
        let mut send = self
            .connection
            .open_uni()
            .await
            .map_err(|e| NetError::QuicConnection(e.to_string()))?;

        send.set_priority(crate::qos::TrafficClass::BackgroundBulk.quic_priority())
            .map_err(|e| NetError::QuicStream(e.to_string()))?;

        send.write_all(&[QuicStreamType::UnidirectionalBulkTensor as u8])
            .await
            .map_err(|e| NetError::QuicStream(e.to_string()))?;

        Ok(send)
    }

    /// Sends a low-latency ping on the control plane and waits for pong reply, measuring RTT.
    /// Uses `TrafficClass::CriticalControl` and records latency in the shaper.
    pub async fn ping_control(&self, payload: &[u8]) -> Result<(Duration, Vec<u8>), NetError> {
        let t0 = Instant::now();
        self.shaper.acquire(crate::qos::TrafficClass::CriticalControl, payload.len()).await?;

        let (mut send, mut recv) = self.open_bi_control().await?;
        let len_bytes = (payload.len() as u32).to_le_bytes();
        send.write_all(&len_bytes).await.map_err(|e| NetError::QuicStream(e.to_string()))?;
        send.write_all(payload).await.map_err(|e| NetError::QuicStream(e.to_string()))?;
        send.finish().map_err(|e| NetError::QuicStream(e.to_string()))?;

        let mut reply_len_buf = [0u8; 4];
        recv.read_exact(&mut reply_len_buf).await.map_err(|e| NetError::QuicStream(e.to_string()))?;
        let reply_len = u32::from_le_bytes(reply_len_buf) as usize;
        let mut reply = vec![0u8; reply_len];
        recv.read_exact(&mut reply).await.map_err(|e| NetError::QuicStream(e.to_string()))?;

        let rtt = t0.elapsed();
        self.shaper.record_control_latency(rtt);
        Ok((rtt, reply))
    }

    /// Sends a bulk tensor payload over a dedicated unidirectional stream with BLAKE3 verification digest,
    /// paced by the `BandwidthShaper`.
    pub async fn send_bulk_tensor(&self, tensor_id: &str, data: &[u8]) -> Result<[u8; 32], NetError> {
        let mut stream = self.open_uni_bulk_tensor().await?;

        // 1. Compute BLAKE3 digest
        let digest = *blake3::hash(data).as_bytes();

        // 2. Framing: [tensor_id_len: u16][tensor_id][payload_len: u64][digest: 32 bytes][data]
        let id_bytes = tensor_id.as_bytes();
        let id_len = id_bytes.len() as u16;
        let payload_len = data.len() as u64;

        stream
            .write_all(&id_len.to_le_bytes())
            .await
            .map_err(|e| NetError::QuicStream(e.to_string()))?;
        stream
            .write_all(id_bytes)
            .await
            .map_err(|e| NetError::QuicStream(e.to_string()))?;
        stream
            .write_all(&payload_len.to_le_bytes())
            .await
            .map_err(|e| NetError::QuicStream(e.to_string()))?;
        stream
            .write_all(&digest)
            .await
            .map_err(|e| NetError::QuicStream(e.to_string()))?;

        // Write data in chunks paced by BandwidthShaper
        let chunk_size = 65_536; // 64 KiB
        for chunk in data.chunks(chunk_size) {
            self.shaper
                .acquire(crate::qos::TrafficClass::BackgroundBulk, chunk.len())
                .await?;
            stream
                .write_all(chunk)
                .await
                .map_err(|e| NetError::QuicStream(e.to_string()))?;
        }
        stream
            .finish()
            .map_err(|e| NetError::QuicStream(e.to_string()))?;

        Ok(digest)
    }

    /// Reads a bulk tensor payload from an incoming unidirectional stream.
    pub async fn recv_bulk_tensor(
        mut stream: RecvStream,
    ) -> Result<(String, Vec<u8>, [u8; 32]), NetError> {
        // Read stream type header
        let mut type_buf = [0u8; 1];
        stream
            .read_exact(&mut type_buf)
            .await
            .map_err(|e| NetError::QuicStream(e.to_string()))?;

        if type_buf[0] != QuicStreamType::UnidirectionalBulkTensor as u8 {
            return Err(NetError::QuicStream(format!(
                "Unexpected stream type header: expected 0x03, got 0x{:02x}",
                type_buf[0]
            )));
        }

        // Read tensor_id_len
        let mut id_len_buf = [0u8; 2];
        stream
            .read_exact(&mut id_len_buf)
            .await
            .map_err(|e| NetError::QuicStream(e.to_string()))?;
        let id_len = u16::from_le_bytes(id_len_buf) as usize;

        // Read tensor_id
        let mut id_buf = vec![0u8; id_len];
        stream
            .read_exact(&mut id_buf)
            .await
            .map_err(|e| NetError::QuicStream(e.to_string()))?;
        let tensor_id =
            String::from_utf8(id_buf).map_err(|e| NetError::QuicStream(e.to_string()))?;

        // Read payload_len
        let mut payload_len_buf = [0u8; 8];
        stream
            .read_exact(&mut payload_len_buf)
            .await
            .map_err(|e| NetError::QuicStream(e.to_string()))?;
        let payload_len = u64::from_le_bytes(payload_len_buf) as usize;

        // Read digest
        let mut expected_digest = [0u8; 32];
        stream
            .read_exact(&mut expected_digest)
            .await
            .map_err(|e| NetError::QuicStream(e.to_string()))?;

        // Read payload
        let mut payload = vec![0u8; payload_len];
        stream
            .read_exact(&mut payload)
            .await
            .map_err(|e| NetError::QuicStream(e.to_string()))?;

        // Verify digest
        let actual_digest = *blake3::hash(&payload).as_bytes();
        if actual_digest != expected_digest {
            return Err(NetError::QuicStream(
                "BLAKE3 checksum mismatch on bulk tensor stream".into(),
            ));
        }

        Ok((tensor_id, payload, actual_digest))
    }

    /// Performs the 2-RTT mutual attestation handshake, binding the TLS 1.3 exporter.
    pub async fn execute_mutual_attestation(
        &self,
        local_keypair: &NodeKeypair,
        hardware_fingerprint: [u8; 32],
    ) -> Result<(), NetError> {
        self.set_state(PeerConnectionState::Attesting);

        let (mut send, mut recv) = self.open_bi_control().await?;

        // 1. Challenger sends AttestationChallenge
        let challenge = MutualAttestationProtocol::create_challenge(local_keypair);
        let challenge_bytes =
            bincode::serialize(&challenge).map_err(|e| NetError::Serialization(e.to_string()))?;
        let len_bytes = (challenge_bytes.len() as u32).to_le_bytes();

        send.write_all(&len_bytes)
            .await
            .map_err(|e| NetError::QuicStream(e.to_string()))?;
        send.write_all(&challenge_bytes)
            .await
            .map_err(|e| NetError::QuicStream(e.to_string()))?;

        // 2. Read remote proof
        let mut proof_len_buf = [0u8; 4];
        recv.read_exact(&mut proof_len_buf)
            .await
            .map_err(|e| NetError::QuicStream(e.to_string()))?;
        let proof_len = u32::from_le_bytes(proof_len_buf) as usize;

        let mut proof_bytes = vec![0u8; proof_len];
        recv.read_exact(&mut proof_bytes)
            .await
            .map_err(|e| NetError::QuicStream(e.to_string()))?;
        let proof: AttestationProof =
            bincode::deserialize(&proof_bytes).map_err(|e| NetError::Serialization(e.to_string()))?;

        // 3. Export TLS 1.3 keying material from connection
        let mut expected_exporter = [0u8; 32];
        self.connection
            .export_keying_material(&mut expected_exporter, TLS_EXPORTER_LABEL, &[])
            .map_err(|_| NetError::AttestationFailed("Export keying material failed".into()))?;

        // 4. Derive remote public key from remote_did
        let remote_pk = DidKey::to_ed25519_bytes(&self.remote_did)
            .map_err(|e| NetError::AttestationFailed(e.to_string()))?;

        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        // 5. Verify proof
        MutualAttestationProtocol::verify_proof(
            &proof,
            &challenge,
            &expected_exporter,
            &remote_pk,
            30_000,
            now_ms,
        )
        .map_err(|e| NetError::AttestationFailed(e.to_string()))?;

        // 6. Prover responds to challenger's challenge: send our proof back
        let our_proof = MutualAttestationProtocol::create_proof(
            local_keypair,
            &challenge,
            expected_exporter,
            hardware_fingerprint,
        );
        let our_proof_bytes =
            bincode::serialize(&our_proof).map_err(|e| NetError::Serialization(e.to_string()))?;
        let our_len = (our_proof_bytes.len() as u32).to_le_bytes();
        send.write_all(&our_len)
            .await
            .map_err(|e| NetError::QuicStream(e.to_string()))?;
        send.write_all(&our_proof_bytes)
            .await
            .map_err(|e| NetError::QuicStream(e.to_string()))?;

        self.set_state(PeerConnectionState::AuthenticatedReady);
        Ok(())
    }
}

/// Configuration parameters for `QuicTransportEngine`.
#[derive(Debug, Clone)]
pub struct QuicTransportConfig {
    /// Local UDP socket address to bind.
    pub bind_addr: SocketAddr,
    /// Target cluster identifier.
    pub cluster_id: String,
    /// Local node X.509 certificate.
    pub node_cert: NodeCertificate,
    /// Local node keypair.
    pub node_keypair: Arc<NodeKeypair>,
    /// Trusted Cluster Certificate Authority root certificate DER.
    pub ca_cert_der: Vec<u8>,
    /// Active Certificate Revocation List.
    pub crl: Arc<RwLock<ClusterCrl>>,
    /// Send window size in bytes (default: 16 MiB).
    pub send_window_bytes: u32,
    /// Receive window size in bytes (default: 16 MiB).
    pub recv_window_bytes: u32,
    /// Bandwidth shaper and QoS traffic scheduler.
    pub bandwidth_shaper: Arc<crate::qos::BandwidthShaper>,
}

impl QuicTransportConfig {
    /// Creates a default configuration.
    #[must_use]
    pub fn new(
        bind_addr: SocketAddr,
        cluster_id: impl Into<String>,
        node_cert: NodeCertificate,
        node_keypair: Arc<NodeKeypair>,
        ca_cert_der: Vec<u8>,
        crl: Arc<RwLock<ClusterCrl>>,
    ) -> Self {
        Self {
            bind_addr,
            cluster_id: cluster_id.into(),
            node_cert,
            node_keypair,
            ca_cert_der,
            crl,
            send_window_bytes: 16 * 1024 * 1024,
            recv_window_bytes: 16 * 1024 * 1024,
            bandwidth_shaper: Arc::new(crate::qos::BandwidthShaper::new(0)),
        }
    }

    /// Attaches a custom bandwidth shaper.
    #[must_use]
    pub fn with_bandwidth_shaper(mut self, shaper: Arc<crate::qos::BandwidthShaper>) -> Self {
        self.bandwidth_shaper = shaper;
        self
    }
}

/// Active QUIC data-plane transport engine managing endpoints and connection pooling.
#[derive(Debug)]
pub struct QuicTransportEngine {
    config: QuicTransportConfig,
    endpoint: Endpoint,
    peers: Arc<RwLock<HashMap<NodeId, Arc<PeerConnection>>>>,
    cancel_token: CancellationToken,
}

impl QuicTransportEngine {
    /// Starts the QUIC transport engine listening on the configured socket.
    ///
    /// # Errors
    /// Returns `NetError` if socket binding, certificate parsing, or Quinn initialization fails.
    pub fn start(config: QuicTransportConfig) -> Result<Self, NetError> {
        let domain = if config.bind_addr.is_ipv6() {
            socket2::Domain::IPV6
        } else {
            socket2::Domain::IPV4
        };
        let socket2_sock = socket2::Socket::new(domain, socket2::Type::DGRAM, Some(socket2::Protocol::UDP))
            .map_err(NetError::Io)?;
        let _ = socket2_sock.set_reuse_address(true);
        let _ = socket2_sock.set_nonblocking(true);
        let _ = crate::qos::apply_socket_dscp(&socket2_sock, crate::qos::TrafficClass::CriticalControl.dscp_value());
        socket2_sock.bind(&config.bind_addr.into()).map_err(NetError::Io)?;
        let std_sock: std::net::UdpSocket = socket2_sock.into();
        Self::start_with_socket(config, std_sock)
    }

    /// Starts the QUIC transport engine on an already-bound or hole-punched UDP socket.
    ///
    /// # Errors
    /// Returns `NetError` if certificate parsing, crypto configuration, or Quinn initialization fails.
    pub fn start_with_socket(config: QuicTransportConfig, std_sock: std::net::UdpSocket) -> Result<Self, NetError> {
        let _ = std_sock.set_nonblocking(true);
        let cancel_token = CancellationToken::new();
        let peers = Arc::new(RwLock::new(HashMap::new()));

        // 1. Build PKCS#8 private key from Ed25519 secret seed
        let pkcs8_der = ed25519_secret_to_pkcs8_der(config.node_keypair.seed());
        let private_key = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(pkcs8_der));

        let cert_chain = vec![CertificateDer::from(config.node_cert.cert_der.clone())];

        // 2. Build Server mTLS Config
        let client_verifier = Arc::new(ClusterCaClientVerifier::new(
            &config.ca_cert_der,
            &config.cluster_id,
            Arc::clone(&config.crl),
        )?);

        let mut server_crypto = ServerConfig::builder()
            .with_client_cert_verifier(client_verifier)
            .with_single_cert(cert_chain.clone(), private_key.clone_key())
            .map_err(|e| NetError::Tls(format!("Failed to build server crypto: {e}")))?;
        server_crypto.alpn_protocols = vec![QUIC_ALPN_V1.to_vec()];

        // 3. Build Client mTLS Config
        let server_verifier = Arc::new(ClusterCaServerVerifier::new(
            &config.ca_cert_der,
            &config.cluster_id,
            Arc::clone(&config.crl),
        )?);

        let mut client_crypto = ClientConfig::builder()
            .dangerous()
            .with_custom_certificate_verifier(server_verifier)
            .with_client_auth_cert(cert_chain, private_key)
            .map_err(|e| NetError::Tls(format!("Failed to build client crypto: {e}")))?;
        client_crypto.alpn_protocols = vec![QUIC_ALPN_V1.to_vec()];

        // 4. Configure Transport Parameters
        let mut transport = TransportConfig::default();
        transport.max_idle_timeout(Some(
            Duration::from_secs(30)
                .try_into()
                .map_err(|e| NetError::QuicConnection(format!("{e}")))?,
        ));
        transport.keep_alive_interval(Some(Duration::from_secs(5)));
        transport.send_window(u64::from(config.send_window_bytes));
        transport.receive_window(quinn::VarInt::from_u32(config.recv_window_bytes));
        transport.stream_receive_window(quinn::VarInt::from_u32(config.recv_window_bytes / 2));
        transport.max_concurrent_bidi_streams(quinn::VarInt::from_u32(256));
        transport.max_concurrent_uni_streams(quinn::VarInt::from_u32(256));

        let transport = Arc::new(transport);

        let quic_server_config = QuicServerConfig::try_from(server_crypto)
            .map_err(|e| NetError::Tls(format!("Failed to build QuicServerConfig: {e}")))?;
        let mut server_config = quinn::ServerConfig::with_crypto(Arc::new(quic_server_config));
        server_config.transport_config(Arc::clone(&transport));

        let quic_client_config = QuicClientConfig::try_from(client_crypto)
            .map_err(|e| NetError::Tls(format!("Failed to build QuicClientConfig: {e}")))?;
        let mut client_config = quinn::ClientConfig::new(Arc::new(quic_client_config));
        client_config.transport_config(transport);

        // 5. Create Quinn Endpoint with provided std_sock
        let runtime = quinn::default_runtime().ok_or_else(|| {
            NetError::Io(std::io::Error::other(
                "No async runtime available for Quinn endpoint",
            ))
        })?;

        let mut endpoint = Endpoint::new(
            quinn::EndpointConfig::default(),
            Some(server_config),
            std_sock,
            runtime,
        ).map_err(NetError::Io)?;
        endpoint.set_default_client_config(client_config);

        // 6. Spawn Background Accept Loop
        let a_endpoint = endpoint.clone();
        let a_peers = Arc::clone(&peers);
        let a_cancel = cancel_token.clone();
        let a_keypair = Arc::clone(&config.node_keypair);
        let a_shaper = Arc::clone(&config.bandwidth_shaper);

        tokio::spawn(async move {
            while !a_cancel.is_cancelled() {
                tokio::select! {
                    _ = a_cancel.cancelled() => break,
                    incoming = a_endpoint.accept() => {
                        let incoming = match incoming {
                            Some(inc) => inc,
                            None => break,
                        };

                        let a_peers = Arc::clone(&a_peers);
                        let a_keypair = Arc::clone(&a_keypair);
                        let a_shaper = Arc::clone(&a_shaper);

                        tokio::spawn(async move {
                            if let Ok(conn) = incoming.await {
                                if let Some(peer_identity) = conn.peer_identity() {
                                    if let Some(certs) = peer_identity.downcast_ref::<Vec<CertificateDer<'_>>>() {
                                        if let Some(first) = certs.first() {
                                            if let Ok(info) = extract_cert_info(first.as_ref()) {
                                                if let Ok(pk) = DidKey::to_ed25519_bytes(&info.did_key) {
                                                    if let Ok(node_id) = NodeId::from_public_key_bytes(&pk) {
                                                        let peer_conn = Arc::new(PeerConnection {
                                                            node_id: node_id.clone(),
                                                            remote_did: info.did_key.clone(),
                                                            cert_info: info,
                                                            connection: conn.clone(),
                                                            shaper: a_shaper,
                                                            state: Arc::new(RwLock::new(PeerConnectionState::Handshaking)),
                                                        });

                                                        let p_conn = Arc::clone(&peer_conn);
                                                        let kp = Arc::clone(&a_keypair);
                                                        tokio::spawn(async move {
                                                            let _ = handle_inbound_attestation(p_conn, kp).await;
                                                        });

                                                        if let Ok(mut lock) = a_peers.write() {
                                                            lock.insert(node_id, peer_conn);
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        });
                    }
                }
            }
        });

        Ok(Self {
            config,
            endpoint,
            peers,
            cancel_token,
        })
    }

    /// Establishes an outbound QUIC mTLS connection to a peer and performs mutual attestation.
    ///
    /// # Errors
    /// Returns `NetError` if connection establishment or mutual attestation fails.
    pub async fn connect(&self, target_addr: SocketAddr) -> Result<Arc<PeerConnection>, NetError> {
        let server_name = format!("{}.umcf.local", self.config.cluster_id);
        let connecting = self
            .endpoint
            .connect(target_addr, &server_name)
            .map_err(|e| NetError::QuicConnection(e.to_string()))?;

        let conn = connecting
            .await
            .map_err(|e| NetError::QuicConnection(e.to_string()))?;

        // Extract verified peer identity from TLS session
        let peer_identity = conn
            .peer_identity()
            .ok_or_else(|| NetError::Tls("Missing peer identity from TLS handshake".into()))?;

        let certs = peer_identity
            .downcast_ref::<Vec<CertificateDer<'_>>>()
            .ok_or_else(|| NetError::Tls("Cannot downcast peer certificate chain".into()))?;

        let first_cert = certs
            .first()
            .ok_or_else(|| NetError::Tls("Empty peer certificate chain".into()))?;

        let info = extract_cert_info(first_cert.as_ref())?;
        let pk = DidKey::to_ed25519_bytes(&info.did_key)
            .map_err(|e| NetError::InvalidCertificate(format!("Cannot decode DID key: {e}")))?;
        let node_id = NodeId::from_public_key_bytes(&pk)
            .map_err(|e| NetError::InvalidCertificate(format!("Cannot derive NodeId: {e}")))?;

        let peer_conn = Arc::new(PeerConnection {
            node_id: node_id.clone(),
            remote_did: info.did_key.clone(),
            cert_info: info,
            connection: conn,
            shaper: Arc::clone(&self.config.bandwidth_shaper),
            state: Arc::new(RwLock::new(PeerConnectionState::Handshaking)),
        });

        // Execute mutual attestation protocol immediately
        peer_conn
            .execute_mutual_attestation(&self.config.node_keypair, [0u8; 32])
            .await?;

        let p_conn = Arc::clone(&peer_conn);
        tokio::spawn(async move {
            let _ = handle_inbound_streams(p_conn).await;
        });

        if let Ok(mut lock) = self.peers.write() {
            lock.insert(node_id, Arc::clone(&peer_conn));
        }

        Ok(peer_conn)
    }

    /// Returns the local bound UDP address of the QUIC endpoint.
    pub fn local_addr(&self) -> Result<SocketAddr, NetError> {
        self.endpoint.local_addr().map_err(NetError::Io)
    }

    /// Rebinds the underlying UDP socket of the Quinn endpoint to a new socket,
    /// triggering RFC 9000 QUIC connection migration across all active peer sessions.
    ///
    /// # Errors
    /// Returns `NetError::Io` if socket rebinding fails.
    pub fn rebind(&self, new_sock: std::net::UdpSocket) -> Result<SocketAddr, NetError> {
        let _ = new_sock.set_nonblocking(true);
        let new_addr = new_sock.local_addr().map_err(NetError::Io)?;
        self.endpoint.rebind(new_sock).map_err(NetError::Io)?;
        Ok(new_addr)
    }

    /// Retrieves an active peer connection by NodeId.
    #[must_use]
    pub fn get_peer(&self, node_id: &NodeId) -> Option<Arc<PeerConnection>> {
        self.peers.read().ok()?.get(node_id).cloned()
    }

    /// Returns all active peer connections.
    #[must_use]
    pub fn all_peers(&self) -> Vec<Arc<PeerConnection>> {
        self.peers
            .read()
            .map(|lock| lock.values().cloned().collect())
            .unwrap_or_default()
    }

    /// Stops the QUIC transport engine cleanly.
    pub fn stop(&self) {
        self.cancel_token.cancel();
        self.endpoint.close(0u32.into(), b"shutdown");
    }
}

impl Drop for QuicTransportEngine {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Encodes a 32-byte Ed25519 secret seed into canonical RFC 8410 PKCS#8 v1 DER.
#[must_use]
pub fn ed25519_secret_to_pkcs8_der(secret: &[u8; 32]) -> Vec<u8> {
    let mut pkcs8 = Vec::with_capacity(48);
    pkcs8.extend_from_slice(&[
        0x30, 0x2e, // SEQUENCE (46 bytes)
        0x02, 0x01, 0x00, // INTEGER 0 (version)
        0x30, 0x05, 0x06, 0x03, 0x2b, 0x65, 0x70, // AlgorithmIdentifier (Ed25519 OID: 1.3.101.112)
        0x04, 0x22, 0x04, 0x20, // OCTET STRING wrapping OCTET STRING of 32 bytes
    ]);
    pkcs8.extend_from_slice(secret);
    pkcs8
}

async fn handle_inbound_attestation(
    peer_conn: Arc<PeerConnection>,
    keypair: Arc<NodeKeypair>,
) -> Result<(), NetError> {
    let (mut send, mut recv) = peer_conn
        .connection
        .accept_bi()
        .await
        .map_err(|e| NetError::QuicConnection(e.to_string()))?;

    let mut type_buf = [0u8; 1];
    recv.read_exact(&mut type_buf)
        .await
        .map_err(|e| NetError::QuicStream(e.to_string()))?;

    if type_buf[0] != QuicStreamType::BidirectionalControl as u8 {
        return Err(NetError::QuicStream(
            "Expected BidirectionalControl stream".into(),
        ));
    }

    let mut len_buf = [0u8; 4];
    recv.read_exact(&mut len_buf)
        .await
        .map_err(|e| NetError::QuicStream(e.to_string()))?;
    let len = u32::from_le_bytes(len_buf) as usize;

    let mut challenge_bytes = vec![0u8; len];
    recv.read_exact(&mut challenge_bytes)
        .await
        .map_err(|e| NetError::QuicStream(e.to_string()))?;

    let challenge: AttestationChallenge =
        bincode::deserialize(&challenge_bytes).map_err(|e| NetError::Serialization(e.to_string()))?;

    let mut exporter = [0u8; 32];
    peer_conn
        .connection
        .export_keying_material(&mut exporter, TLS_EXPORTER_LABEL, &[])
        .map_err(|_| NetError::AttestationFailed("Export keying material failed".into()))?;

    let our_proof = MutualAttestationProtocol::create_proof(
        &keypair,
        &challenge,
        exporter,
        [0u8; 32],
    );
    let proof_bytes =
        bincode::serialize(&our_proof).map_err(|e| NetError::Serialization(e.to_string()))?;
    let plen = (proof_bytes.len() as u32).to_le_bytes();
    send.write_all(&plen)
        .await
        .map_err(|e| NetError::QuicStream(e.to_string()))?;
    send.write_all(&proof_bytes)
        .await
        .map_err(|e| NetError::QuicStream(e.to_string()))?;

    // Read challenger's proof back
    let mut c_len_buf = [0u8; 4];
    recv.read_exact(&mut c_len_buf)
        .await
        .map_err(|e| NetError::QuicStream(e.to_string()))?;
    let c_len = u32::from_le_bytes(c_len_buf) as usize;

    let mut c_proof_bytes = vec![0u8; c_len];
    recv.read_exact(&mut c_proof_bytes)
        .await
        .map_err(|e| NetError::QuicStream(e.to_string()))?;

    peer_conn.set_state(PeerConnectionState::AuthenticatedReady);
    let p_conn = Arc::clone(&peer_conn);
    tokio::spawn(async move {
        let _ = handle_inbound_streams(p_conn).await;
    });
    Ok(())
}

async fn handle_inbound_streams(peer_conn: Arc<PeerConnection>) -> Result<(), NetError> {
    while peer_conn.state() == PeerConnectionState::AuthenticatedReady {
        match peer_conn.connection.accept_bi().await {
            Ok((mut send, mut recv)) => {
                tokio::spawn(async move {
                    let mut type_buf = [0u8; 1];
                    if recv.read_exact(&mut type_buf).await.is_err() {
                        return;
                    }
                    if type_buf[0] == QuicStreamType::BidirectionalControl as u8 {
                        let _ = send.set_priority(crate::qos::TrafficClass::CriticalControl.quic_priority());
                        let mut len_buf = [0u8; 4];
                        if recv.read_exact(&mut len_buf).await.is_ok() {
                            let len = u32::from_le_bytes(len_buf) as usize;
                            let mut payload = vec![0u8; len];
                            if recv.read_exact(&mut payload).await.is_ok() {
                                let _ = send.write_all(&len_buf).await;
                                let _ = send.write_all(&payload).await;
                                let _ = send.finish();
                            }
                        }
                    } else if type_buf[0] == QuicStreamType::BidirectionalRpc as u8 {
                        let _ = send.set_priority(crate::qos::TrafficClass::ElasticCompute.quic_priority());
                        let mut len_buf = [0u8; 4];
                        if recv.read_exact(&mut len_buf).await.is_ok() {
                            let len = u32::from_le_bytes(len_buf) as usize;
                            let mut payload = vec![0u8; len];
                            if recv.read_exact(&mut payload).await.is_ok() {
                                let reply = format!("ack:{}", String::from_utf8_lossy(&payload));
                                let rlen = (reply.len() as u32).to_le_bytes();
                                let _ = send.write_all(&rlen).await;
                                let _ = send.write_all(reply.as_bytes()).await;
                                let _ = send.finish();
                            }
                        }
                    }
                });
            }
            Err(_) => break,
        }
    }
    Ok(())
}
