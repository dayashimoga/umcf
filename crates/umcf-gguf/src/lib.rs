//! Universal Mobile Compute Fabric (UMCF) GGUF Crate.
//!
//! Provides content-addressed GGUF model storage, resumable P2P shard distribution,
//! zero-copy metadata inspection, memory footprint estimation, cryptographic provenance,
//! and licensing / RBAC security gates.
//!
//! Strictly adheres to `#![forbid(unsafe_code)]` and zero-placeholder invariants.

#![forbid(unsafe_code)]

pub mod error;
pub mod estimator;
pub mod license;
pub mod p2p_sync;
pub mod parser;
pub mod security;
pub mod store;

// Re-exports for store and error modules
pub use error::{StoreError, StoreErrorCode, SyncError, SyncErrorCode};
pub use estimator::{
    CompatibilityVerdict, DeviceCapabilities, HardwareCompatibilityValidator, InferenceParams,
    LayerMemoryBreakdown, MemoryFootprintReport, ModelFootprintEstimator, OsPlatform,
};
pub use license::{LicenseAcceptanceReceipt, LicenseCategory, LicensePolicyEngine};
pub use p2p_sync::{
    Bitfield, DownloadScope, MockPeerProvider, PeerChunkProvider, PeerScore,
    ResumableShardDownloader, SyncProgress, SyncStatus,
};
pub use parser::{
    create_test_gguf_bytes, GgmlType, GgufMetadataValue, GgufParser, GgufTensorInfo,
    LayerWeightProfile, ModelArchitectureMetadata, ModelInspection, GGUF_MAGIC, GGUF_VERSION_V2,
    GGUF_VERSION_V3,
};
pub use security::{
    Ed25519PublicKey, ModelAccessPolicy, ModelIntegrityVerifier, ModelQuarantineManager,
    ModelVisibility, SignedModelAttestation, VerifiedModelGuard, VerifyMode,
};
pub use store::{
    compute_merkle_root, ByteRange, ChunkDescriptor, ChunkHash, ContentAddressedModelStore,
    Ed25519Signature, LayerRangeHandle, ModelId, ModelManifest, VerifiedModelMmap,
    CHUNK_SIZE_BYTES,
};
