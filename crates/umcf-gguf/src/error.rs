//! Error types and error codes for UMCF GGUF store and P2P sync.
//! Strictly enforces `#![forbid(unsafe_code)]` and zero placeholders.

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::store::{ByteRange, ModelId};

/// Machine-readable error codes for model store failures.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StoreErrorCode {
    IoFailure,
    ModelNotFound,
    ModelAlreadyExists,
    HashMismatch,
    ChunkHashMismatch,
    CorruptedFile,
    RangeOutOfBounds,
    ModelInUse,
    InvalidManifest,
    BitfieldCorrupted,
    InvalidSignature,
}

impl std::fmt::Display for StoreErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IoFailure => write!(f, "ERR_STORE_IO_FAILURE"),
            Self::ModelNotFound => write!(f, "ERR_STORE_MODEL_NOT_FOUND"),
            Self::ModelAlreadyExists => write!(f, "ERR_STORE_MODEL_ALREADY_EXISTS"),
            Self::HashMismatch => write!(f, "ERR_STORE_HASH_MISMATCH"),
            Self::ChunkHashMismatch => write!(f, "ERR_STORE_CHUNK_HASH_MISMATCH"),
            Self::CorruptedFile => write!(f, "ERR_STORE_CORRUPTED_FILE"),
            Self::RangeOutOfBounds => write!(f, "ERR_STORE_RANGE_OUT_OF_BOUNDS"),
            Self::ModelInUse => write!(f, "ERR_STORE_MODEL_IN_USE"),
            Self::InvalidManifest => write!(f, "ERR_STORE_INVALID_MANIFEST"),
            Self::BitfieldCorrupted => write!(f, "ERR_STORE_BITFIELD_CORRUPTED"),
            Self::InvalidSignature => write!(f, "ERR_STORE_INVALID_SIGNATURE"),
        }
    }
}

/// Errors occurring during model storage, ingestion, verification, or eviction.
#[derive(Debug, Error)]
pub enum StoreError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Model not found in store: {0}")]
    ModelNotFound(ModelId),

    #[error("Model already exists in store: {0}")]
    ModelAlreadyExists(ModelId),

    #[error("Content hash mismatch: expected {expected}, actual {actual}")]
    HashMismatch { expected: String, actual: String },

    #[error("Chunk {index} hash mismatch: expected {expected}, actual {actual}")]
    ChunkHashMismatch {
        index: u32,
        expected: String,
        actual: String,
    },

    #[error("Corrupted file: {0}")]
    CorruptedFile(String),

    #[error("Range {requested:?} is out of bounds for model of size {total} bytes")]
    RangeOutOfBounds { requested: ByteRange, total: u64 },

    #[error("Model is currently in use: {0}")]
    InUse(ModelId),

    #[error("Invalid manifest: {0}")]
    InvalidManifest(String),

    #[error("Bitfield corrupted: {0}")]
    BitfieldCorrupted(String),

    #[error("Invalid publisher signature: {0}")]
    InvalidSignature(String),
}

impl StoreError {
    #[must_use]
    pub fn error_code(&self) -> StoreErrorCode {
        match self {
            Self::Io(_) => StoreErrorCode::IoFailure,
            Self::ModelNotFound(_) => StoreErrorCode::ModelNotFound,
            Self::ModelAlreadyExists(_) => StoreErrorCode::ModelAlreadyExists,
            Self::HashMismatch { .. } => StoreErrorCode::HashMismatch,
            Self::ChunkHashMismatch { .. } => StoreErrorCode::ChunkHashMismatch,
            Self::CorruptedFile(_) => StoreErrorCode::CorruptedFile,
            Self::RangeOutOfBounds { .. } => StoreErrorCode::RangeOutOfBounds,
            Self::InUse(_) => StoreErrorCode::ModelInUse,
            Self::InvalidManifest(_) => StoreErrorCode::InvalidManifest,
            Self::BitfieldCorrupted(_) => StoreErrorCode::BitfieldCorrupted,
            Self::InvalidSignature(_) => StoreErrorCode::InvalidSignature,
        }
    }

    #[must_use]
    pub fn code(&self) -> StoreErrorCode {
        self.error_code()
    }
}

/// Machine-readable error codes for P2P sync failures.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SyncErrorCode {
    AllPeersFailed,
    PeerUnavailable,
    PeerCorruptedChunk,
    TransferCancelled,
    TransferPaused,
    TransferTimeout,
    StorageFailure,
    ProtocolViolation,
}

impl std::fmt::Display for SyncErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AllPeersFailed => write!(f, "ERR_SYNC_ALL_PEERS_FAILED"),
            Self::PeerUnavailable => write!(f, "ERR_SYNC_PEER_UNAVAILABLE"),
            Self::PeerCorruptedChunk => write!(f, "ERR_SYNC_PEER_CORRUPTED_CHUNK"),
            Self::TransferCancelled => write!(f, "ERR_SYNC_TRANSFER_CANCELLED"),
            Self::TransferPaused => write!(f, "ERR_SYNC_TRANSFER_PAUSED"),
            Self::TransferTimeout => write!(f, "ERR_SYNC_TRANSFER_TIMEOUT"),
            Self::StorageFailure => write!(f, "ERR_SYNC_STORAGE_FAILURE"),
            Self::ProtocolViolation => write!(f, "ERR_SYNC_PROTOCOL_VIOLATION"),
        }
    }
}

/// Errors occurring during P2P shard synchronization.
#[derive(Debug, Error)]
pub enum SyncError {
    #[error("All peers in swarm failed or exhausted for chunk {chunk_index}")]
    AllPeersFailed { chunk_index: u32 },

    #[error("Peer {peer_id} unavailable: {reason}")]
    PeerUnavailable { peer_id: String, reason: String },

    #[error("Peer {peer_id} sent corrupted chunk {chunk_index}")]
    PeerCorruptedChunk { peer_id: String, chunk_index: u32 },

    #[error("Sync operation was cancelled")]
    Cancelled,

    #[error("Sync operation is currently paused")]
    Paused,

    #[error("Timed out fetching chunk {chunk_index} after {elapsed_ms}ms")]
    Timeout { chunk_index: u32, elapsed_ms: u64 },

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Underlying storage error: {0}")]
    Store(#[from] StoreError),

    #[error("Protocol violation: {0}")]
    Protocol(String),
}

impl SyncError {
    #[must_use]
    pub fn error_code(&self) -> SyncErrorCode {
        match self {
            Self::AllPeersFailed { .. } => SyncErrorCode::AllPeersFailed,
            Self::PeerUnavailable { .. } => SyncErrorCode::PeerUnavailable,
            Self::PeerCorruptedChunk { .. } => SyncErrorCode::PeerCorruptedChunk,
            Self::Cancelled => SyncErrorCode::TransferCancelled,
            Self::Paused => SyncErrorCode::TransferPaused,
            Self::Timeout { .. } => SyncErrorCode::TransferTimeout,
            Self::Io(_) => SyncErrorCode::StorageFailure,
            Self::Store(_) => SyncErrorCode::StorageFailure,
            Self::Protocol(_) => SyncErrorCode::ProtocolViolation,
        }
    }

    #[must_use]
    pub fn code(&self) -> SyncErrorCode {
        self.error_code()
    }
}
