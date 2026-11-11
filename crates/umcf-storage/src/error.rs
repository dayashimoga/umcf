//! Error taxonomy for UMCF Content-Addressed Storage & Quota Enforcer (UMCF S7.P1.SP1).

use thiserror::Error;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("storage quota exceeded: requested {requested_bytes} B, available {available_bytes} B (effective limit: {effective_limit_bytes} B)")]
    QuotaExceeded {
        requested_bytes: u64,
        available_bytes: u64,
        effective_limit_bytes: u64,
    },

    #[error("host OS free disk floor protection triggered: free space {host_free_bytes} B below required safety floor {safety_floor_bytes} B")]
    HostSpaceExhausted {
        host_free_bytes: u64,
        safety_floor_bytes: u64,
    },

    #[error("chunk not found: {chunk_id}")]
    ChunkNotFound { chunk_id: String },

    #[error("chunk integrity verification mismatch: expected {expected_id}, computed {computed_id}")]
    ChunkIntegrityMismatch {
        expected_id: String,
        computed_id: String,
    },

    #[error("object DAG manifest not found: {root_cid}")]
    ManifestNotFound { root_cid: String },

    #[error("object DAG manifest corrupted or malformed: {reason}")]
    ManifestCorrupted { reason: String },

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}
