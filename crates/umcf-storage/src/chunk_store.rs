//! Gear-Hash FastCDC & BLAKE3 Content-Addressed Chunk Store (UMCF S7.P1.SP1).
//!
//! Provides deterministic Content-Defined Chunking (FastCDC), 2-level fanout chunk persistence,
//! BLAKE3 content-addressed integrity verification, and startup recovery reconciliation.

use std::collections::{BTreeMap, HashMap};
use std::fmt;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

static STAGING_COUNTER: AtomicU64 = AtomicU64::new(1);

use serde::{Deserialize, Serialize};

use crate::error::StorageError;
use crate::quota::{QuotaReservationPermit, StrictQuotaEnforcer};

/// 256-entry deterministic Gear Table for FastCDC rolling hash.
pub const GEAR_TABLE: [u64; 256] = {
    let mut table = [0u64; 256];
    let mut state = 0x2f928e469e32a67au64;
    let mut i = 0;
    while i < 256 {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        table[i] = state;
        i += 1;
    }
    table
};

/// 32-byte BLAKE3 hash digest identifying an immutable chunk.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ChunkId(pub [u8; 32]);

impl ChunkId {
    #[must_use]
    pub fn from_bytes(data: &[u8]) -> Self {
        let hash = blake3::hash(data);
        Self(*hash.as_bytes())
    }

    #[must_use]
    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }

    #[must_use]
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    pub fn from_hex(s: &str) -> Result<Self, StorageError> {
        let bytes = hex::decode(s).map_err(|e| StorageError::ManifestCorrupted {
            reason: format!("invalid chunk hex {s}: {e}"),
        })?;
        if bytes.len() != 32 {
            return Err(StorageError::ManifestCorrupted {
                reason: format!("invalid chunk id byte length {}", bytes.len()),
            });
        }
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&bytes);
        Ok(Self(arr))
    }
}

impl fmt::Debug for ChunkId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ChunkId({})", self.to_hex())
    }
}

impl fmt::Display for ChunkId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

/// 32-byte BLAKE3 root manifest digest.
pub type ObjectCid = ChunkId;

/// Reference to a chunk within a larger object DAG.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChunkRef {
    pub chunk_id: ChunkId,
    pub offset: u64,
    pub length: u32,
}

/// Hierarchical manifest describing an object constructed from content-addressed chunks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObjectDagManifest {
    pub root_cid: ObjectCid,
    pub total_size_bytes: u64,
    pub chunk_refs: Vec<ChunkRef>,
    pub content_type: String,
    pub metadata: BTreeMap<String, String>,
    pub created_at_ms: u64,
}

impl ObjectDagManifest {
    /// Computes the deterministic root CID for this manifest.
    #[must_use]
    pub fn compute_cid(
        total_size_bytes: u64,
        chunk_refs: &[ChunkRef],
        content_type: &str,
        metadata: &BTreeMap<String, String>,
        created_at_ms: u64,
    ) -> ObjectCid {
        let mut hasher = blake3::Hasher::new();
        hasher.update(&total_size_bytes.to_le_bytes());
        hasher.update(&(chunk_refs.len() as u64).to_le_bytes());
        for cr in chunk_refs {
            hasher.update(&cr.chunk_id.0);
            hasher.update(&cr.offset.to_le_bytes());
            hasher.update(&cr.length.to_le_bytes());
        }
        hasher.update(content_type.as_bytes());
        for (k, v) in metadata {
            hasher.update(k.as_bytes());
            hasher.update(v.as_bytes());
        }
        hasher.update(&created_at_ms.to_le_bytes());
        ChunkId(*hasher.finalize().as_bytes())
    }
}

/// Configuration parameters for FastCDC chunking.
#[derive(Debug, Clone)]
pub struct FastCdcConfig {
    pub min_size: usize,
    pub avg_size: usize,
    pub max_size: usize,
    pub fixed_size: Option<usize>,
}

impl Default for FastCdcConfig {
    fn default() -> Self {
        Self::default_production()
    }
}

impl FastCdcConfig {
    #[must_use]
    pub fn default_production() -> Self {
        Self {
            min_size: 262_144,   // 256 KiB
            avg_size: 1_048_576, // 1 MiB
            max_size: 4_194_304, // 4 MiB
            fixed_size: None,
        }
    }

    #[must_use]
    pub fn custom(min_size: usize, avg_size: usize, max_size: usize) -> Self {
        Self {
            min_size: min_size.max(64),
            avg_size: avg_size.max(min_size),
            max_size: max_size.max(avg_size),
            fixed_size: None,
        }
    }

    #[must_use]
    pub fn fixed(chunk_size: usize) -> Self {
        Self {
            min_size: chunk_size,
            avg_size: chunk_size,
            max_size: chunk_size,
            fixed_size: Some(chunk_size),
        }
    }

    /// Partitions an arbitrary byte buffer into chunk boundaries using FastCDC.
    #[must_use]
    pub fn chunk_data<'a>(&self, data: &'a [u8]) -> Vec<&'a [u8]> {
        if data.is_empty() {
            return Vec::new();
        }

        if let Some(fixed_sz) = self.fixed_size {
            let mut chunks = Vec::new();
            let mut offset = 0;
            while offset < data.len() {
                let end = (offset + fixed_sz).min(data.len());
                chunks.push(&data[offset..end]);
                offset = end;
            }
            return chunks;
        }

        if data.len() <= self.min_size {
            return vec![data];
        }

        let mut chunks = Vec::new();
        let mut cursor = 0;

        let bits = (self.avg_size as f64).log2().round() as u32;
        let mask_s = (1u64 << (bits + 1)) - 1;
        let mask_l = (1u64 << (bits - 1)) - 1;

        while cursor < data.len() {
            let remaining = data.len() - cursor;
            if remaining <= self.min_size {
                chunks.push(&data[cursor..]);
                break;
            }

            let max_cut = cursor + self.max_size.min(remaining);
            let mut split_point = max_cut;
            let mut hash = 0u64;

            let mut i = cursor + self.min_size;
            let mid = cursor + self.avg_size.min(remaining);

            // Phase 1: min_size .. avg_size with mask_s
            while i < mid && i < max_cut {
                hash = (hash << 1).wrapping_add(GEAR_TABLE[data[i] as usize]);
                if (hash & mask_s) == 0 {
                    split_point = i + 1;
                    break;
                }
                i += 1;
            }

            // Phase 2: avg_size .. max_size with mask_l if not split yet
            if split_point == max_cut && i < max_cut {
                while i < max_cut {
                    hash = (hash << 1).wrapping_add(GEAR_TABLE[data[i] as usize]);
                    if (hash & mask_l) == 0 {
                        split_point = i + 1;
                        break;
                    }
                    i += 1;
                }
            }

            chunks.push(&data[cursor..split_point]);
            cursor = split_point;
        }

        chunks
    }
}

/// In-memory metadata record for an individual chunk in the store.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkMetadata {
    pub chunk_id: ChunkId,
    pub size_bytes: u64,
    pub ref_count: u32,
    pub pinned: bool,
    pub last_access_us: u64,
    pub created_at_us: u64,
}

/// Options for object ingestion into the content-addressed chunk store.
#[derive(Debug, Clone)]
pub struct IngestOptions<'a> {
    pub content_type: &'a str,
    pub metadata: BTreeMap<String, String>,
    pub chunk_config: FastCdcConfig,
    pub host_free_bytes: u64,
    pub pinned: bool,
}

impl<'a> Default for IngestOptions<'a> {
    fn default() -> Self {
        Self {
            content_type: "application/octet-stream",
            metadata: BTreeMap::new(),
            chunk_config: FastCdcConfig::default_production(),
            host_free_bytes: 10 * 1024 * 1024 * 1024, // 10 GiB default
            pinned: false,
        }
    }
}

/// Content-Addressed Chunk Store on disk.
#[derive(Debug)]
pub struct ContentAddressedChunkStore {
    pub root_dir: PathBuf,
    pub chunks_dir: PathBuf,
    pub staging_dir: PathBuf,
    pub manifests_dir: PathBuf,
    pub index: Arc<Mutex<HashMap<ChunkId, ChunkMetadata>>>,
}

impl ContentAddressedChunkStore {
    /// Initializes or opens a chunk store in the given directory.
    pub fn new(root_dir: impl Into<PathBuf>) -> Result<Self, StorageError> {
        let root = root_dir.into();
        let chunks_dir = root.join("chunks");
        let staging_dir = root.join("staging");
        let manifests_dir = root.join("manifests");

        fs::create_dir_all(&chunks_dir)?;
        fs::create_dir_all(&staging_dir)?;
        fs::create_dir_all(&manifests_dir)?;

        let store = Self {
            root_dir: root,
            chunks_dir,
            staging_dir,
            manifests_dir,
            index: Arc::new(Mutex::new(HashMap::new())),
        };

        // Reconcile index from disk on startup
        let _ = store.startup_recovery()?;

        Ok(store)
    }

    /// Computes the 2-level fanout filesystem path: `chunks/{hex[0..2]}/{hex[2..4]}/{full_hex}.chk`.
    #[must_use]
    pub fn chunk_path(&self, chunk_id: &ChunkId) -> PathBuf {
        let hex = chunk_id.to_hex();
        self.chunks_dir
            .join(&hex[0..2])
            .join(&hex[2..4])
            .join(format!("{hex}.chk"))
    }

    /// Checks if a chunk is already resident on disk and indexed.
    #[must_use]
    pub fn has_chunk(&self, chunk_id: &ChunkId) -> bool {
        let index = match self.index.lock() {
            Ok(idx) => idx,
            Err(poisoned) => poisoned.into_inner(),
        };
        if index.contains_key(chunk_id) {
            return true;
        }
        self.chunk_path(chunk_id).exists()
    }

    /// Stores a chunk with atomic write-to-staging + rename and deduplication.
    pub fn store_chunk(
        &self,
        data: &[u8],
        permit: &mut QuotaReservationPermit,
        pinned: bool,
    ) -> Result<ChunkId, StorageError> {
        let chunk_id = ChunkId::from_bytes(data);
        let path = self.chunk_path(&chunk_id);
        let now_us = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_micros() as u64;

        let mut index = match self.index.lock() {
            Ok(idx) => idx,
            Err(poisoned) => poisoned.into_inner(),
        };

        // Deduplication: if chunk already exists, increment ref count and do NOT rewrite
        if let Some(meta) = index.get_mut(&chunk_id) {
            meta.ref_count += 1;
            meta.last_access_us = now_us;
            if pinned {
                meta.pinned = true;
            }
            // Do not consume additional disk bytes
            permit.commit(0);
            return Ok(chunk_id);
        }

        if path.exists() {
            index.insert(
                chunk_id,
                ChunkMetadata {
                    chunk_id,
                    size_bytes: data.len() as u64,
                    ref_count: 1,
                    pinned,
                    last_access_us: now_us,
                    created_at_us: now_us,
                },
            );
            permit.commit(0);
            return Ok(chunk_id);
        }

        // Write new chunk via staging file
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let nonce = STAGING_COUNTER.fetch_add(1, Ordering::Relaxed);
        let staging_path = self.staging_dir.join(format!(
            ".staging.{}.{}.{}",
            chunk_id.to_hex(),
            now_us,
            nonce
        ));

        {
            let mut file = File::create(&staging_path)?;
            file.write_all(data)?;
            file.sync_all()?;
        }

        // Atomic rename to destination
        fs::rename(&staging_path, &path)?;

        // Update index and commit permit
        index.insert(
            chunk_id,
            ChunkMetadata {
                chunk_id,
                size_bytes: data.len() as u64,
                ref_count: 1,
                pinned,
                last_access_us: now_us,
                created_at_us: now_us,
            },
        );

        permit.commit(data.len() as u64);
        Ok(chunk_id)
    }

    /// Reads and verifies a chunk by its ChunkId.
    pub fn read_chunk(&self, chunk_id: &ChunkId) -> Result<Vec<u8>, StorageError> {
        let path = self.chunk_path(chunk_id);
        if !path.exists() {
            return Err(StorageError::ChunkNotFound {
                chunk_id: chunk_id.to_hex(),
            });
        }

        let mut file = File::open(&path)?;
        let mut buf = Vec::new();
        file.read_to_end(&mut buf)?;

        // Cryptographic integrity verification
        let computed = ChunkId::from_bytes(&buf);
        if computed != *chunk_id {
            return Err(StorageError::ChunkIntegrityMismatch {
                expected_id: chunk_id.to_hex(),
                computed_id: computed.to_hex(),
            });
        }

        // Update last access
        let now_us = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_micros() as u64;

        let mut index = match self.index.lock() {
            Ok(idx) => idx,
            Err(poisoned) => poisoned.into_inner(),
        };
        if let Some(meta) = index.get_mut(chunk_id) {
            meta.last_access_us = now_us;
        }

        Ok(buf)
    }

    /// Ingests a full file, chunks it with FastCDC, reserves quota, and outputs an ObjectDagManifest.
    pub fn ingest_object(
        &self,
        data: &[u8],
        options: IngestOptions<'_>,
        quota_enforcer: &Arc<StrictQuotaEnforcer>,
    ) -> Result<ObjectDagManifest, StorageError> {
        let slices = options.chunk_config.chunk_data(data);
        let mut chunk_refs = Vec::with_capacity(slices.len());
        let mut current_offset = 0u64;

        for slice in slices {
            let chunk_len = slice.len() as u32;
            let mut permit =
                quota_enforcer.try_reserve(chunk_len as u64, options.host_free_bytes)?;

            let cid = self.store_chunk(slice, &mut permit, options.pinned)?;

            chunk_refs.push(ChunkRef {
                chunk_id: cid,
                offset: current_offset,
                length: chunk_len,
            });
            current_offset += chunk_len as u64;
        }

        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        let root_cid = ObjectDagManifest::compute_cid(
            data.len() as u64,
            &chunk_refs,
            options.content_type,
            &options.metadata,
            now_ms,
        );

        let manifest = ObjectDagManifest {
            root_cid,
            total_size_bytes: data.len() as u64,
            chunk_refs,
            content_type: options.content_type.to_string(),
            metadata: options.metadata,
            created_at_ms: now_ms,
        };

        self.store_manifest(&manifest)?;
        Ok(manifest)
    }

    /// Reconstructs the original object from its manifest.
    pub fn reconstruct_object(
        &self,
        manifest: &ObjectDagManifest,
    ) -> Result<Vec<u8>, StorageError> {
        let mut full_bytes = Vec::with_capacity(manifest.total_size_bytes as usize);
        for cr in &manifest.chunk_refs {
            let chunk_bytes = self.read_chunk(&cr.chunk_id)?;
            full_bytes.extend_from_slice(&chunk_bytes);
        }
        Ok(full_bytes)
    }

    /// Persists an ObjectDagManifest to disk.
    pub fn store_manifest(&self, manifest: &ObjectDagManifest) -> Result<(), StorageError> {
        let path = self
            .manifests_dir
            .join(format!("{}.manifest.json", manifest.root_cid.to_hex()));
        let json = serde_json::to_string_pretty(manifest).map_err(|e| {
            StorageError::ManifestCorrupted {
                reason: e.to_string(),
            }
        })?;
        fs::write(path, json)?;
        Ok(())
    }

    /// Reads an ObjectDagManifest from disk.
    pub fn read_manifest(&self, root_cid: &ObjectCid) -> Result<ObjectDagManifest, StorageError> {
        let path = self
            .manifests_dir
            .join(format!("{}.manifest.json", root_cid.to_hex()));
        if !path.exists() {
            return Err(StorageError::ManifestNotFound {
                root_cid: root_cid.to_hex(),
            });
        }
        let content = fs::read_to_string(path)?;
        let manifest: ObjectDagManifest =
            serde_json::from_str(&content).map_err(|e| StorageError::ManifestCorrupted {
                reason: e.to_string(),
            })?;
        Ok(manifest)
    }

    /// Cleans orphan `.staging` files left by crashes/aborts and reconciles total used bytes.
    pub fn startup_recovery(&self) -> Result<(usize, u64), StorageError> {
        let mut cleaned_staging_files = 0;
        if self.staging_dir.exists() {
            for entry in fs::read_dir(&self.staging_dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.is_file() {
                    let _ = fs::remove_file(path);
                    cleaned_staging_files += 1;
                }
            }
        }

        let mut total_used_bytes = 0u64;
        let mut index = match self.index.lock() {
            Ok(idx) => idx,
            Err(poisoned) => poisoned.into_inner(),
        };

        if self.chunks_dir.exists() {
            let mut stack = vec![self.chunks_dir.clone()];
            while let Some(dir) = stack.pop() {
                for entry in fs::read_dir(dir)? {
                    let entry = entry?;
                    let p = entry.path();
                    if p.is_dir() {
                        stack.push(p);
                    } else if p.is_file() && p.extension().and_then(|s| s.to_str()) == Some("chk") {
                        if let Some(file_name) = p.file_stem().and_then(|s| s.to_str()) {
                            if let Ok(cid) = ChunkId::from_hex(file_name) {
                                if let Ok(meta) = p.metadata() {
                                    let len = meta.len();
                                    total_used_bytes += len;
                                    index.entry(cid).or_insert_with(|| ChunkMetadata {
                                        chunk_id: cid,
                                        size_bytes: len,
                                        ref_count: 1,
                                        pinned: false,
                                        last_access_us: 0,
                                        created_at_us: 0,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok((cleaned_staging_files, total_used_bytes))
    }

    /// Evicts unpinned chunks up to target bytes based on LRU order.
    pub fn evict_unpinned_chunks(&self, target_bytes: u64) -> Result<u64, StorageError> {
        let mut index = match self.index.lock() {
            Ok(idx) => idx,
            Err(poisoned) => poisoned.into_inner(),
        };

        let mut unpinned: Vec<(ChunkId, u64, u64)> = index
            .iter()
            .filter(|(_, meta)| !meta.pinned)
            .map(|(cid, meta)| (*cid, meta.size_bytes, meta.last_access_us))
            .collect();

        // Sort by last_access_us ascending (least recently accessed first)
        unpinned.sort_by_key(|&(_, _, ts)| ts);

        let mut freed_bytes = 0u64;
        for (cid, sz, _) in unpinned {
            if freed_bytes >= target_bytes {
                break;
            }
            let path = self.chunk_path(&cid);
            if path.exists() {
                let _ = fs::remove_file(path);
            }
            index.remove(&cid);
            freed_bytes += sz;
        }

        Ok(freed_bytes)
    }

    /// Returns a snapshot of metadata for all chunks currently tracked in the index.
    #[must_use]
    pub fn list_all_chunks(&self) -> Vec<ChunkMetadata> {
        let index = match self.index.lock() {
            Ok(idx) => idx,
            Err(poisoned) => poisoned.into_inner(),
        };
        index.values().cloned().collect()
    }

    /// Purges a chunk from the index and deletes its on-disk .chk file.
    /// Returns the number of bytes reclaimed.
    pub fn purge_chunk(&self, chunk_id: &ChunkId) -> Result<u64, StorageError> {
        let mut index = match self.index.lock() {
            Ok(idx) => idx,
            Err(poisoned) => poisoned.into_inner(),
        };

        if let Some(meta) = index.remove(chunk_id) {
            let path = self.chunk_path(chunk_id);
            if path.exists() {
                let _ = fs::remove_file(path);
            }
            Ok(meta.size_bytes)
        } else {
            Ok(0)
        }
    }

    /// Restores a corrupted chunk by writing verified healthy bytes directly to its on-disk path.
    pub fn restore_corrupted_chunk(
        &self,
        chunk_id: &ChunkId,
        data: &[u8],
    ) -> Result<(), StorageError> {
        let path = self.chunk_path(chunk_id);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut file = File::create(&path)?;
        file.write_all(data)?;
        file.sync_all()?;

        let mut index = match self.index.lock() {
            Ok(idx) => idx,
            Err(poisoned) => poisoned.into_inner(),
        };
        let now_us = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_micros() as u64;

        index.insert(
            *chunk_id,
            ChunkMetadata {
                chunk_id: *chunk_id,
                size_bytes: data.len() as u64,
                ref_count: 1,
                pinned: false,
                last_access_us: now_us,
                created_at_us: now_us,
            },
        );

        Ok(())
    }
}
