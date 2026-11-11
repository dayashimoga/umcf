//! Universal Mobile Compute Fabric (UMCF) Content-Addressed Storage Subsystem.
//!
//! Provides:
//! - FastCDC Content-Defined Chunking with Gear hash rolling checksums.
//! - BLAKE3 content-addressed chunk store with 2-level fanout persistence.
//! - Byte-exact atomic quota enforcement with lock-free compare-and-swap reservation permits.
//! - Host OS disk space safety floor protection (15% safety floor).
//! - Deduplication, LRU cache eviction, and startup crash recovery.

#![forbid(unsafe_code)]

pub mod chunk_store;
pub mod error;
pub mod quota;

pub use chunk_store::{
    ChunkId, ChunkMetadata, ChunkRef, ContentAddressedChunkStore, FastCdcConfig, IngestOptions,
    ObjectCid, ObjectDagManifest,
};
pub use error::StorageError;
pub use quota::{QuotaReservationPermit, StrictQuotaEnforcer};
