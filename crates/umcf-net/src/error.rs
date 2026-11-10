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

    #[error("Network engine shutdown")]
    Shutdown,
}

