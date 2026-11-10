//! Universal Mobile Compute Fabric (UMCF) GGUF Crate.
//!
//! Provides content-addressed GGUF model storage, resumable P2P shard distribution,
//! zero-copy metadata inspection, and memory footprint estimation.
//!
//! Strictly adheres to `#![forbid(unsafe_code)]` and zero-placeholder invariants.

#![forbid(unsafe_code)]

pub mod error;
pub mod estimator;
pub mod parser;
pub mod p2p_sync;
pub mod store;

// Re-exports for store and error modules
pub use error::{StoreError, StoreErrorCode, SyncError, SyncErrorCode};
pub use estimator::{
    CompatibilityVerdict, DeviceCapabilities, HardwareCompatibilityValidator, InferenceParams,
    LayerMemoryBreakdown, MemoryFootprintReport, ModelFootprintEstimator, OsPlatform,
};
pub use p2p_sync::{
    Bitfield, DownloadScope, MockPeerProvider, PeerChunkProvider, PeerScore,
    ResumableShardDownloader, SyncProgress, SyncStatus,
};
pub use parser::{
    create_test_gguf_bytes, GgmlType, GgufMetadataValue, GgufParser, GgufTensorInfo,
    LayerWeightProfile, ModelArchitectureMetadata, ModelInspection, GGUF_MAGIC,
    GGUF_VERSION_V2, GGUF_VERSION_V3,
};
pub use store::{
    compute_merkle_root, ByteRange, ChunkDescriptor, ChunkHash, ContentAddressedModelStore,
    Ed25519Signature, LayerRangeHandle, ModelId, ModelManifest, VerifiedModelMmap,
    CHUNK_SIZE_BYTES,
};
