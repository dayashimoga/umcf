//! `umcf-net` error taxonomy and diagnostic classifications.
//! Strictly enforces `#![forbid(unsafe_code)]` and zero-placeholder invariants.

use thiserror::Error;

/// Network transport, discovery, and protocol errors in UMCF.
#[derive(Debug, Error)]
pub enum NetError {
    #[error("I/O error during network operation: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization/Deserialization error: {0}")]
    Serialization(String),

    #[error("Invalid discovery beacon: {0}")]
    InvalidBeacon(String),

    #[error("Beacon rejected: cluster mismatch (expected '{expected}', got '{actual}')")]
    ClusterMismatch {
        expected: String,
        actual: String,
    },

    #[error("Cryptographic signature verification failed for node '{node_id}'")]
    SignatureVerificationFailed {
        node_id: String,
    },

    #[error("Replay protection: beacon with timestamp {timestamp} or seq {seq} already processed")]
    ReplayDetected {
        timestamp: u64,
        seq: u64,
    },

    #[error("Rate limit exceeded for peer '{addr}': {count} beacons received")]
    RateLimited {
        addr: String,
        count: usize,
    },

    #[error("QUIC connection error: {0}")]
    QuicConnection(String),

    #[error("QUIC stream error: {0}")]
    QuicStream(String),

    #[error("TLS configuration or handshake error: {0}")]
    Tls(String),

    #[error("Invalid certificate: {0}")]
    InvalidCertificate(String),

    #[error("Certificate revoked by ClusterCrl: serial '{serial}', did '{did}'")]
    CertificateRevoked {
        serial: String,
        did: String,
    },

    #[error("Mutual attestation failed: {0}")]
    AttestationFailed(String),

    #[error("Invalid STUN message: {0}")]
    InvalidStun(String),

    #[error("Network operation timed out: {0}")]
    Timeout(String),

    #[error("Security alert during network traversal: {0}")]
    SecurityAlert(String),

    #[error("UDP hole punch failed: {0}")]
    HolePunchFailed(String),

    #[error("Network engine shutdown")]
    Shutdown,
}

