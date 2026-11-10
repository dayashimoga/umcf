//! `umcf-registry` error types and diagnostic classifications.
//! Strictly enforces `#![forbid(unsafe_code)]` and zero-placeholder invariants.

use thiserror::Error;

use crate::hlc::HlcError;

/// Registry and persistent storage errors.
/// Large error types are boxed to maintain minimal enum footprint.
#[derive(Debug, Error)]
pub enum RegistryError {
    #[error("Database error: {0}")]
    Database(Box<redb::DatabaseError>),

    #[error("Transaction error: {0}")]
    Transaction(Box<redb::TransactionError>),

    #[error("Table error: {0}")]
    Table(Box<redb::TableError>),

    #[error("Storage error: {0}")]
    Storage(Box<redb::StorageError>),

    #[error("Commit error: {0}")]
    Commit(Box<redb::CommitError>),

    #[error("Compaction error: {0}")]
    Compaction(Box<redb::CompactionError>),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("HLC error: {0}")]
    Hlc(#[from] HlcError),

    #[error("Node not found: {0}")]
    NodeNotFound(String),

    #[error("Corrupted entry in table '{table}': {details}")]
    CorruptedEntry {
        table: &'static str,
        details: String,
    },

    #[error("Clock drift exceeded limit: {0} ms")]
    ClockDrift(u64),

    #[error("Registry IO error: {0}")]
    Io(#[from] std::io::Error),
}

impl From<redb::DatabaseError> for RegistryError {
    fn from(e: redb::DatabaseError) -> Self {
        Self::Database(Box::new(e))
    }
}

impl From<redb::TransactionError> for RegistryError {
    fn from(e: redb::TransactionError) -> Self {
        Self::Transaction(Box::new(e))
    }
}

impl From<redb::TableError> for RegistryError {
    fn from(e: redb::TableError) -> Self {
        Self::Table(Box::new(e))
    }
}

impl From<redb::StorageError> for RegistryError {
    fn from(e: redb::StorageError) -> Self {
        Self::Storage(Box::new(e))
    }
}

impl From<redb::CommitError> for RegistryError {
    fn from(e: redb::CommitError) -> Self {
        Self::Commit(Box::new(e))
    }
}

impl From<redb::CompactionError> for RegistryError {
    fn from(e: redb::CompactionError) -> Self {
        Self::Compaction(Box::new(e))
    }
}
