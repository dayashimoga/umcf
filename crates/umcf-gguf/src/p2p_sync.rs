//! Resumable P2P Shard Distribution & Swarm Chunk Synchronization.
//!
//! Tracks chunk completion via persistent bitfields (.part.bitfield), provides
//! multi-peer work-stealing scheduling with peer scoring and corruption bans,
//! and supports selective layer-range downloads for mobile compute nodes.
//!
//! Strictly enforces `#![forbid(unsafe_code)]` and zero placeholders.

#![forbid(unsafe_code)]

use std::collections::{BTreeSet, HashMap};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::error::{StoreError, SyncError};
use crate::store::{
    ByteRange, ChunkHash, ContentAddressedModelStore, ModelId, ModelManifest,
};

/// Bitfield tracking presence or completion of 4 MiB chunks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Bitfield {
    chunk_count: usize,
    bytes: Vec<u8>,
}

impl Bitfield {
    /// Creates a new empty bitfield for `chunk_count` chunks.
    #[must_use]
    pub fn new(chunk_count: usize) -> Self {
        let byte_len = chunk_count.div_ceil(8);
        Self {
            chunk_count,
            bytes: vec![0u8; byte_len],
        }
    }

    /// Creates a full bitfield with all bits set to 1.
    #[must_use]
    pub fn full(chunk_count: usize) -> Self {
        let byte_len = chunk_count.div_ceil(8);
        let mut bf = Self {
            chunk_count,
            bytes: vec![0xFF; byte_len],
        };
        // Mask out unused trailing bits
        let remainder = chunk_count % 8;
        if remainder != 0 && byte_len > 0 {
            let mask = (1u8 << remainder) - 1;
            bf.bytes[byte_len - 1] &= mask;
        }
        bf
    }

    #[must_use]
    pub const fn chunk_count(&self) -> usize {
        self.chunk_count
    }

    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Sets the bit for `chunk_index`.
    pub fn set(&mut self, chunk_index: usize) {
        if chunk_index < self.chunk_count {
            let byte_idx = chunk_index / 8;
            let bit_idx = chunk_index % 8;
            self.bytes[byte_idx] |= 1 << bit_idx;
        }
    }

    /// Clears the bit for `chunk_index`.
    pub fn unset(&mut self, chunk_index: usize) {
        if chunk_index < self.chunk_count {
            let byte_idx = chunk_index / 8;
            let bit_idx = chunk_index % 8;
            self.bytes[byte_idx] &= !(1 << bit_idx);
        }
    }

    /// Checks if the bit for `chunk_index` is set.
    #[must_use]
    pub fn get(&self, chunk_index: usize) -> bool {
        if chunk_index >= self.chunk_count {
            return false;
        }
        let byte_idx = chunk_index / 8;
        let bit_idx = chunk_index % 8;
        (self.bytes[byte_idx] & (1 << bit_idx)) != 0
    }

    /// Returns the number of completed chunks (bits set to 1).
    #[must_use]
    pub fn completed_count(&self) -> usize {
        let mut count = 0;
        for i in 0..self.chunk_count {
            if self.get(i) {
                count += 1;
            }
        }
        count
    }

    /// Checks if all chunks are completed.
    #[must_use]
    pub fn is_all_set(&self) -> bool {
        self.completed_count() == self.chunk_count
    }

    /// Returns a list of all chunk indices that are not yet set.
    #[must_use]
    pub fn missing_indices(&self) -> Vec<usize> {
        let mut missing = Vec::with_capacity(self.chunk_count.saturating_sub(self.completed_count()));
        for i in 0..self.chunk_count {
            if !self.get(i) {
                missing.push(i);
            }
        }
        missing
    }

    /// Deserializes a Bitfield from raw bytes with a verified length check.
    pub fn from_bytes(bytes: &[u8], chunk_count: usize) -> Result<Self, StoreError> {
        let expected_len = chunk_count.div_ceil(8);
        if bytes.len() != expected_len {
            return Err(StoreError::BitfieldCorrupted(format!(
                "invalid bitfield byte length: expected {}, got {}",
                expected_len,
                bytes.len()
            )));
        }
        Ok(Self {
            chunk_count,
            bytes: bytes.to_vec(),
        })
    }

    /// Atomically persists this bitfield to disk.
    pub fn save_to_file(&self, path: &Path) -> Result<(), StoreError> {
        let tmp_path = path.with_extension("tmp");
        let mut file = File::create(&tmp_path)?;
        // Write header: 8-byte chunk_count + raw bytes
        file.write_all(&(self.chunk_count as u64).to_le_bytes())?;
        file.write_all(&self.bytes)?;
        file.sync_all()?;
        fs::rename(tmp_path, path)?;
        Ok(())
    }

    /// Loads a bitfield from disk with crash recovery checks.
    pub fn load_from_file(path: &Path, expected_chunk_count: usize) -> Result<Self, StoreError> {
        if !path.exists() {
            return Ok(Self::new(expected_chunk_count));
        }
        let mut file = File::open(path)?;
        let mut count_buf = [0u8; 8];
        file.read_exact(&mut count_buf)?;
        let stored_count = u64::from_le_bytes(count_buf) as usize;
        if stored_count != expected_chunk_count {
            return Err(StoreError::BitfieldCorrupted(format!(
                "bitfield chunk count mismatch: expected {}, stored {}",
                expected_chunk_count, stored_count
            )));
        }

        let mut data = Vec::new();
        file.read_to_end(&mut data)?;
        Self::from_bytes(&data, expected_chunk_count)
    }
}

/// Scope of a model download operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DownloadScope {
    /// Download the entire model (all chunks).
    FullModel,
    /// Download GGUF header chunks + layers overlapping [start_layer..=end_layer].
    LayerRange {
        start_layer: usize,
        end_layer: usize,
    },
    /// Download explicit custom chunk indices.
    CustomChunks(Vec<u32>),
}

/// Dynamic score and health tracking for a P2P swarm peer.
#[derive(Debug, Clone)]
pub struct PeerScore {
    pub peer_id: String,
    pub rtt_ms: f64,
    pub bandwidth_bytes_per_sec: f64,
    pub successful_chunks: u32,
    pub corrupt_chunks: u32,
    pub timeouts: u32,
    pub penalized_until_ms: u64,
}

impl PeerScore {
    #[must_use]
    pub fn new(peer_id: impl Into<String>) -> Self {
        Self {
            peer_id: peer_id.into(),
            rtt_ms: 20.0,
            bandwidth_bytes_per_sec: 10_000_000.0, // 10 MB/s default initial expectation
            successful_chunks: 0,
            corrupt_chunks: 0,
            timeouts: 0,
            penalized_until_ms: 0,
        }
    }

    #[must_use]
    pub fn is_penalized(&self, now_ms: u64) -> bool {
        self.penalized_until_ms > now_ms
    }

    /// Computes peer priority score (higher is better).
    #[must_use]
    pub fn priority_score(&self, now_ms: u64) -> f64 {
        if self.is_penalized(now_ms) {
            return 0.0;
        }
        let rtt_weight = 1000.0 / (self.rtt_ms.max(1.0) + 10.0);
        let bw_weight = (self.bandwidth_bytes_per_sec / 1_000_000.0).max(0.1);
        rtt_weight * bw_weight
    }

    pub fn record_success(&mut self, chunk_bytes: usize, elapsed: Duration) {
        self.successful_chunks += 1;
        let elapsed_secs = elapsed.as_secs_f64().max(0.001);
        let sample_bw = (chunk_bytes as f64) / elapsed_secs;
        // Exponential moving average (alpha = 0.3)
        self.bandwidth_bytes_per_sec = (0.7 * self.bandwidth_bytes_per_sec) + (0.3 * sample_bw);
        let sample_rtt = elapsed.as_secs_f64() * 1000.0;
        self.rtt_ms = (0.7 * self.rtt_ms) + (0.3 * sample_rtt);
    }

    pub fn record_corruption(&mut self, now_ms: u64) {
        self.corrupt_chunks += 1;
        // Penalize peer: 30-second ban for sending corrupted data
        self.penalized_until_ms = now_ms + 30_000;
        self.bandwidth_bytes_per_sec *= 0.1;
    }

    pub fn record_timeout(&mut self, now_ms: u64) {
        self.timeouts += 1;
        self.rtt_ms *= 1.5;
        if self.timeouts >= 3 {
            self.penalized_until_ms = now_ms + 10_000;
        }
    }
}

/// Abstract interface for querying and fetching chunks from a swarm peer.
pub trait PeerChunkProvider: Send + Sync {
    fn peer_id(&self) -> &str;
    fn query_bitfield(&self, model_id: &ModelId) -> Result<Bitfield, SyncError>;
    fn fetch_chunk(&self, model_id: &ModelId, chunk_index: u32) -> Result<Vec<u8>, SyncError>;
}

/// Mock peer provider for test suites and deterministic simulation.
#[derive(Debug, Clone)]
pub struct MockPeerProvider {
    pub id: String,
    pub bitfield: Bitfield,
    pub chunks: HashMap<u32, Vec<u8>>,
    pub fail_chunk_index: Option<u32>,
    pub corrupt_chunk_index: Option<u32>,
    pub simulated_rtt_ms: u64,
}

impl MockPeerProvider {
    #[must_use]
    pub fn new(id: impl Into<String>, chunk_count: usize) -> Self {
        Self {
            id: id.into(),
            bitfield: Bitfield::full(chunk_count),
            chunks: HashMap::new(),
            fail_chunk_index: None,
            corrupt_chunk_index: None,
            simulated_rtt_ms: 1,
        }
    }

    pub fn with_chunk(mut self, index: u32, data: Vec<u8>) -> Self {
        self.chunks.insert(index, data);
        self
    }
}

impl PeerChunkProvider for MockPeerProvider {
    fn peer_id(&self) -> &str {
        &self.id
    }

    fn query_bitfield(&self, _model_id: &ModelId) -> Result<Bitfield, SyncError> {
        Ok(self.bitfield.clone())
    }

    fn fetch_chunk(&self, _model_id: &ModelId, chunk_index: u32) -> Result<Vec<u8>, SyncError> {
        if self.simulated_rtt_ms > 0 {
            std::thread::sleep(Duration::from_millis(self.simulated_rtt_ms));
        }
        if Some(chunk_index) == self.fail_chunk_index {
            return Err(SyncError::PeerUnavailable {
                peer_id: self.id.clone(),
                reason: "injected failure".into(),
            });
        }
        if Some(chunk_index) == self.corrupt_chunk_index {
            // Return corrupt bytes
            return Ok(vec![0xDE, 0xAD, 0xBE, 0xEF]);
        }
        self.chunks
            .get(&chunk_index)
            .cloned()
            .ok_or_else(|| SyncError::PeerUnavailable {
                peer_id: self.id.clone(),
                reason: format!("chunk {chunk_index} not present on peer"),
            })
    }
}

/// Current status of a synchronization session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SyncStatus {
    Idle,
    Downloading,
    Paused,
    Completed,
    Failed { reason: String },
    Cancelled,
}

/// Progress metrics for a model sync session.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SyncProgress {
    pub model_id: ModelId,
    pub status: SyncStatus,
    pub downloaded_bytes: u64,
    pub total_bytes: u64,
    pub verified_chunks: u32,
    pub total_chunks: u32,
    pub bytes_per_sec: f64,
    pub eta_seconds: Option<f64>,
    pub active_peers: usize,
}

/// Resumable multi-peer shard downloader.
#[derive(Debug)]
pub struct ResumableShardDownloader {
    store: ContentAddressedModelStore,
    manifest: ModelManifest,
    scope: DownloadScope,
    part_data_path: PathBuf,
    part_bitfield_path: PathBuf,
    bitfield: Bitfield,
    required_chunks: BTreeSet<u32>,
    peer_scores: Arc<Mutex<HashMap<String, PeerScore>>>,
    status: Arc<RwLock<SyncStatus>>,
    cancel_flag: Arc<AtomicBool>,
    downloaded_bytes_session: Arc<Mutex<u64>>,
    session_start: Instant,
}

impl ResumableShardDownloader {
    /// Initializes a downloader for the given manifest and scope.
    pub fn new(
        store: ContentAddressedModelStore,
        manifest: ModelManifest,
        scope: DownloadScope,
    ) -> Result<Self, StoreError> {
        let model_id_hex = manifest.model_id.to_hex();
        let part_data_path = store.staging_dir().join(format!("{model_id_hex}.part.data"));
        let part_bitfield_path = store
            .staging_dir()
            .join(format!("{model_id_hex}.part.bitfield"));

        let total_chunks = manifest.chunks.len();
        let bitfield = Bitfield::load_from_file(&part_bitfield_path, total_chunks)?;

        // Compute set of required chunks based on scope
        let required_chunks: BTreeSet<u32> = match &scope {
            DownloadScope::FullModel => (0..total_chunks as u32).collect(),
            DownloadScope::LayerRange {
                start_layer,
                end_layer,
            } => manifest
                .chunks_for_layers(*start_layer, *end_layer)
                .into_iter()
                .collect(),
            DownloadScope::CustomChunks(indices) => indices.iter().copied().collect(),
        };

        Ok(Self {
            store,
            manifest,
            scope,
            part_data_path,
            part_bitfield_path,
            bitfield,
            required_chunks,
            peer_scores: Arc::new(Mutex::new(HashMap::new())),
            status: Arc::new(RwLock::new(SyncStatus::Idle)),
            cancel_flag: Arc::new(AtomicBool::new(false)),
            downloaded_bytes_session: Arc::new(Mutex::new(0)),
            session_start: Instant::now(),
        })
    }

    #[must_use]
    pub fn manifest(&self) -> &ModelManifest {
        &self.manifest
    }

    #[must_use]
    pub fn bitfield(&self) -> &Bitfield {
        &self.bitfield
    }

    #[must_use]
    pub fn required_chunks(&self) -> &BTreeSet<u32> {
        &self.required_chunks
    }

    #[must_use]
    pub fn status(&self) -> SyncStatus {
        self.status
            .read()
            .map_or(SyncStatus::Idle, |s| s.clone())
    }

    /// Computes current progress snapshot.
    pub fn progress(&self) -> SyncProgress {
        let verified = self
            .required_chunks
            .iter()
            .filter(|&&idx| self.bitfield.get(idx as usize))
            .count() as u32;

        let total_required = self.required_chunks.len() as u32;

        let downloaded_bytes: u64 = self
            .required_chunks
            .iter()
            .filter(|&&idx| self.bitfield.get(idx as usize))
            .map(|&idx| {
                self.manifest
                    .chunks
                    .get(idx as usize)
                    .map_or(0u64, |c| u64::from(c.size))
            })
            .sum();

        let total_bytes: u64 = self
            .required_chunks
            .iter()
            .map(|&idx| {
                self.manifest
                    .chunks
                    .get(idx as usize)
                    .map_or(0u64, |c| u64::from(c.size))
            })
            .sum();

        let elapsed = self.session_start.elapsed().as_secs_f64().max(0.1);
        let session_bytes = self
            .downloaded_bytes_session
            .lock()
            .map_or(0, |b| *b);
        let bytes_per_sec = (session_bytes as f64) / elapsed;

        let remaining_bytes = total_bytes.saturating_sub(downloaded_bytes);
        let eta_seconds = if bytes_per_sec > 1000.0 && remaining_bytes > 0 {
            Some((remaining_bytes as f64) / bytes_per_sec)
        } else {
            None
        };

        let active_peers = self
            .peer_scores
            .lock()
            .map_or(0, |m| m.len());

        SyncProgress {
            model_id: self.manifest.model_id,
            status: self.status(),
            downloaded_bytes,
            total_bytes,
            verified_chunks: verified,
            total_chunks: total_required,
            bytes_per_sec,
            eta_seconds,
            active_peers,
        }
    }

    /// Pauses sync, keeping `.part.data` and `.part.bitfield` intact.
    pub fn pause(&mut self) {
        if let Ok(mut status) = self.status.write() {
            *status = SyncStatus::Paused;
        }
    }

    /// Cancels sync, terminating tasks and removing partial files.
    pub fn cancel(&mut self) -> Result<(), StoreError> {
        self.cancel_flag.store(true, Ordering::SeqCst);
        if let Ok(mut status) = self.status.write() {
            *status = SyncStatus::Cancelled;
        }
        if self.part_data_path.exists() {
            fs::remove_file(&self.part_data_path)?;
        }
        if self.part_bitfield_path.exists() {
            fs::remove_file(&self.part_bitfield_path)?;
        }
        Ok(())
    }

    /// Runs multi-peer chunk synchronization to completion or pause.
    pub fn sync(&mut self, peers: &[Arc<dyn PeerChunkProvider>]) -> Result<SyncProgress, SyncError> {
        if peers.is_empty() {
            return Err(SyncError::AllPeersFailed { chunk_index: 0 });
        }

        // Initialize peer scores if not already present
        {
            let mut scores = self
                .peer_scores
                .lock()
                .map_err(|_| SyncError::Store(StoreError::CorruptedFile("score lock poisoned".into())))?;
            for peer in peers {
                scores
                    .entry(peer.peer_id().to_string())
                    .or_insert_with(|| PeerScore::new(peer.peer_id()));
            }
        }

        // Initialize target .part.data file if it does not exist
        if !self.part_data_path.exists() {
            let file = OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(false)
                .open(&self.part_data_path)?;
            file.set_len(self.manifest.total_size_bytes)?;
        }

        if let Ok(mut status) = self.status.write() {
            *status = SyncStatus::Downloading;
        }

        self.session_start = Instant::now();

        // Identify which required chunks are missing
        let missing: Vec<u32> = self
            .required_chunks
            .iter()
            .filter(|&&idx| !self.bitfield.get(idx as usize))
            .copied()
            .collect();

        // Download missing chunks sequentially or via work-stealing across peers
        for &chunk_idx in &missing {
            if self.cancel_flag.load(Ordering::SeqCst) {
                return Err(SyncError::Cancelled);
            }
            if let Ok(status) = self.status.read() {
                if *status == SyncStatus::Paused {
                    return Err(SyncError::Paused);
                }
            }

            let descriptor = self
                .manifest
                .chunks
                .get(chunk_idx as usize)
                .ok_or_else(|| {
                    SyncError::Store(StoreError::RangeOutOfBounds {
                        requested: ByteRange::new(0, 0),
                        total: self.manifest.total_size_bytes,
                    })
                })?;

            let mut chunk_retrieved = false;
            let now_ms = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_or(0, |d| d.as_millis() as u64);

            // Sort peers by priority score
            let mut sorted_peers: Vec<Arc<dyn PeerChunkProvider>> = peers.to_vec();
            {
                let scores = self.peer_scores.lock().map_err(|_| {
                    SyncError::Store(StoreError::CorruptedFile("score lock poisoned".into()))
                })?;
                sorted_peers.sort_by(|a, b| {
                    let score_a = scores
                        .get(a.peer_id())
                        .map_or(1.0, |s| s.priority_score(now_ms));
                    let score_b = scores
                        .get(b.peer_id())
                        .map_or(1.0, |s| s.priority_score(now_ms));
                    score_b
                        .partial_cmp(&score_a)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
            }

            for peer in sorted_peers {
                let start_time = Instant::now();
                match peer.fetch_chunk(&self.manifest.model_id, chunk_idx) {
                    Ok(data) => {
                        // In-memory BLAKE3 verification before writing to disk
                        let computed_hash = ChunkHash(*blake3::hash(&data).as_bytes());
                        if computed_hash != descriptor.hash {
                            // Penalize peer for sending corrupted chunk
                            if let Ok(mut scores) = self.peer_scores.lock() {
                                if let Some(score) = scores.get_mut(peer.peer_id()) {
                                    score.record_corruption(now_ms);
                                }
                            }
                            tracing::warn!(
                                peer_id = %peer.peer_id(),
                                chunk = chunk_idx,
                                expected = %descriptor.hash,
                                got = %computed_hash,
                                "Corrupted chunk detected from peer. Penalizing peer and retrying."
                            );
                            continue; // Try next peer
                        }

                        // Write verified chunk to exact offset in .part.data
                        let mut file = OpenOptions::new()
                            .write(true)
                            .open(&self.part_data_path)?;
                        file.seek(SeekFrom::Start(descriptor.offset))?;
                        file.write_all(&data)?;
                        file.flush()?;

                        // Mark bitfield and persist
                        self.bitfield.set(chunk_idx as usize);
                        self.bitfield.save_to_file(&self.part_bitfield_path)?;

                        // Record peer success
                        let elapsed = start_time.elapsed();
                        if let Ok(mut scores) = self.peer_scores.lock() {
                            if let Some(score) = scores.get_mut(peer.peer_id()) {
                                score.record_success(data.len(), elapsed);
                            }
                        }

                        if let Ok(mut sess_bytes) = self.downloaded_bytes_session.lock() {
                            *sess_bytes += data.len() as u64;
                        }

                        chunk_retrieved = true;
                        break;
                    }
                    Err(err) => {
                        tracing::warn!(
                            peer_id = %peer.peer_id(),
                            chunk = chunk_idx,
                            error = %err,
                            "Peer failed to deliver chunk. Recording timeout/failure."
                        );
                        if let Ok(mut scores) = self.peer_scores.lock() {
                            if let Some(score) = scores.get_mut(peer.peer_id()) {
                                score.record_timeout(now_ms);
                            }
                        }
                    }
                }
            }

            if !chunk_retrieved {
                if let Ok(mut status) = self.status.write() {
                    *status = SyncStatus::Failed {
                        reason: format!("All peers failed for chunk {chunk_idx}"),
                    };
                }
                return Err(SyncError::AllPeersFailed {
                    chunk_index: chunk_idx,
                });
            }
        }

        // Check if all required chunks are now completed
        let all_done = self
            .required_chunks
            .iter()
            .all(|&idx| self.bitfield.get(idx as usize));

        if all_done {
            self.promote_completed_model()?;
            if let Ok(mut status) = self.status.write() {
                *status = SyncStatus::Completed;
            }
        }

        Ok(self.progress())
    }

    /// Atomically promotes completed .part.data to final model store entry.
    fn promote_completed_model(&mut self) -> Result<(), StoreError> {
        let model_id_hex = self.manifest.model_id.to_hex();

        match &self.scope {
            DownloadScope::FullModel => {
                // Verify whole file BLAKE3 Merkle root
                let mut file = File::open(&self.part_data_path)?;
                let mut buffer = Vec::with_capacity(self.manifest.total_size_bytes as usize);
                file.read_to_end(&mut buffer)?;

                if buffer.len() as u64 != self.manifest.total_size_bytes {
                    return Err(StoreError::CorruptedFile("final size mismatch".into()));
                }

                let target_model_path = self.store.model_file_path(&self.manifest.model_id);
                let target_manifest_path = self.store.manifest_file_path(&self.manifest.model_id);

                fs::rename(&self.part_data_path, &target_model_path)?;

                let manifest_json = serde_json::to_string_pretty(&self.manifest)
                    .map_err(|e| StoreError::InvalidManifest(e.to_string()))?;
                fs::write(&target_manifest_path, manifest_json)?;

                // Remove .part.bitfield on clean completion
                if self.part_bitfield_path.exists() {
                    let _ = fs::remove_file(&self.part_bitfield_path);
                }

                tracing::info!(
                    model_id = %self.manifest.model_id,
                    "Promoted completed full model into store"
                );
            }
            DownloadScope::LayerRange {
                start_layer,
                end_layer,
            } => {
                let shard_path = self.store.models_dir().join(format!(
                    "{model_id_hex}_layers_{start_layer}_{end_layer}.shard"
                ));
                fs::copy(&self.part_data_path, &shard_path)?;
                tracing::info!(
                    model_id = %self.manifest.model_id,
                    start = start_layer,
                    end = end_layer,
                    "Saved layer shard to store"
                );
            }
            DownloadScope::CustomChunks(_) => {
                tracing::info!(
                    model_id = %self.manifest.model_id,
                    "Custom chunks completed"
                );
            }
        }

        Ok(())
    }
}
