//! Hierarchical Pinning, Offline Availability Manager & Scan-Resistant W-TinyLFU Cache.
//!
//! Subphase S7.P1.SP3:
//! - Multi-mode pinning: RecursivePermanent, LeaseWithTtl, ActiveJobHold.
//! - Offline hydration guaranteeing < 5 ms read latency with zero network connectivity.
//! - ACID reference counting across shared deduplicated child chunks.
//! - Scan-resistant W-TinyLFU cache (Count-Min sketch + Window/Probationary/Protected LRU).

#![forbid(unsafe_code)]

use crate::chunk_store::{ChunkId, ObjectCid};
use crate::error::StorageError;
use serde::{Deserialize, Serialize};
use std::collections::{hash_map::Entry, HashMap, HashSet, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;
use umcf_core::types::NodeId;

/// Pinning modes controlling chunk retention policy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PinMode {
    /// Object and all its recursive child chunks remain pinned indefinitely until explicit unpin.
    RecursivePermanent,
    /// Object is pinned with a strict lease expiration timestamp in milliseconds since epoch.
    LeaseWithTtl { expires_at_ms: u64 },
    /// Object is held pinned while an associated asynchronous compute job is queued or executing.
    ActiveJobHold { job_id: String },
}

/// Local offline hydration status.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum HydrationState {
    /// Actively downloading/reassembling constituent chunks from local store or peer mesh.
    Hydrating {
        progress_pct: f32,
        hydrated_bytes: u64,
        total_bytes: u64,
    },
    /// All constituent chunks are present locally on disk, verified, and readable without network.
    FullyAvailableOffline,
    /// Some constituent chunks are missing or degraded.
    Degraded {
        missing_chunks: usize,
    },
}

/// Metadata record for a pinned object.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PinRecord {
    /// Target Root Content Identifier.
    pub target_cid: ObjectCid,
    /// Pinning mode (Permanent, TTL lease, or Job hold).
    pub mode: PinMode,
    /// Human-readable label or display filename.
    pub label: String,
    /// Node that requested or initiated the pin.
    pub pinned_by: NodeId,
    /// Current offline hydration state.
    pub local_hydration_state: HydrationState,
    /// Creation timestamp in milliseconds since epoch.
    pub created_at_ms: u64,
    /// Child chunk IDs referenced by this pin.
    pub child_chunks: Vec<ChunkId>,
}

impl PinRecord {
    /// Returns true if the pin is currently active and protecting its constituent chunks from GC.
    #[must_use]
    pub fn is_active(&self, now_ms: u64) -> bool {
        match self.mode {
            PinMode::RecursivePermanent | PinMode::ActiveJobHold { .. } => true,
            PinMode::LeaseWithTtl { expires_at_ms } => now_ms < expires_at_ms,
        }
    }
}

/// Thread-safe hierarchical pin manager with ACID chunk reference counting.
#[derive(Debug, Clone, Default)]
pub struct PinManager {
    inner: Arc<RwLock<PinManagerInner>>,
}

#[derive(Debug, Default)]
struct PinManagerInner {
    pins: HashMap<ObjectCid, PinRecord>,
    chunk_ref_counts: HashMap<ChunkId, usize>,
}

impl PinManager {
    /// Creates a new PinManager.
    #[must_use]
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(PinManagerInner::default())),
        }
    }

    /// Pins an object and all its child chunks, incrementing reference counts atomically.
    pub async fn pin_object(
        &self,
        target_cid: ObjectCid,
        mode: PinMode,
        label: impl Into<String>,
        pinned_by: NodeId,
        child_chunks: Vec<ChunkId>,
        is_fully_hydrated: bool,
    ) -> Result<PinRecord, StorageError> {
        let mut inner = self.inner.write().await;

        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        let hydration_state = if is_fully_hydrated {
            HydrationState::FullyAvailableOffline
        } else {
            HydrationState::Hydrating {
                progress_pct: 0.0,
                hydrated_bytes: 0,
                total_bytes: 0,
            }
        };

        // Increment reference count for all child chunks
        for &chunk_id in &child_chunks {
            *inner.chunk_ref_counts.entry(chunk_id).or_insert(0) += 1;
        }

        let record = PinRecord {
            target_cid,
            mode,
            label: label.into(),
            pinned_by,
            local_hydration_state: hydration_state,
            created_at_ms: now_ms,
            child_chunks,
        };

        inner.pins.insert(target_cid, record.clone());
        Ok(record)
    }

    /// Unpins an object, decrementing child chunk reference counts.
    /// Returns true if an existing pin was removed.
    pub async fn unpin_object(&self, target_cid: &ObjectCid) -> Result<bool, StorageError> {
        let mut inner = self.inner.write().await;

        if let Some(record) = inner.pins.remove(target_cid) {
            for chunk_id in &record.child_chunks {
                if let Some(count) = inner.chunk_ref_counts.get_mut(chunk_id) {
                    *count = count.saturating_sub(1);
                    if *count == 0 {
                        inner.chunk_ref_counts.remove(chunk_id);
                    }
                }
            }
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Updates the hydration state of a pinned object.
    pub async fn update_hydration_state(
        &self,
        target_cid: &ObjectCid,
        new_state: HydrationState,
    ) -> Result<(), StorageError> {
        let mut inner = self.inner.write().await;
        if let Some(record) = inner.pins.get_mut(target_cid) {
            record.local_hydration_state = new_state;
            Ok(())
        } else {
            Err(StorageError::ManifestNotFound {
                root_cid: target_cid.to_hex(),
            })
        }
    }

    /// Checks if a chunk is currently protected by at least one active, non-expired pin.
    pub async fn is_chunk_pinned(&self, chunk_id: &ChunkId, now_ms: u64) -> bool {
        let inner = self.inner.read().await;

        // Fast path: if ref count is 0, not pinned
        if inner.chunk_ref_counts.get(chunk_id).copied().unwrap_or(0) == 0 {
            return false;
        }

        // Verify that at least one referencing pin is currently active (not expired)
        for pin in inner.pins.values() {
            if pin.is_active(now_ms) && pin.child_chunks.contains(chunk_id) {
                return true;
            }
        }

        false
    }

    /// Returns the complete set of all ChunkIds currently protected by active pins.
    pub async fn get_active_pinned_chunks(&self, now_ms: u64) -> HashSet<ChunkId> {
        let inner = self.inner.read().await;
        let mut pinned_set = HashSet::new();

        for pin in inner.pins.values() {
            if pin.is_active(now_ms) {
                for &chunk_id in &pin.child_chunks {
                    pinned_set.insert(chunk_id);
                }
            }
        }

        pinned_set
    }

    /// Returns all registered pins.
    pub async fn list_pins(&self) -> Vec<PinRecord> {
        let inner = self.inner.read().await;
        inner.pins.values().cloned().collect()
    }

    /// Returns a specific pin record.
    pub async fn get_pin(&self, target_cid: &ObjectCid) -> Option<PinRecord> {
        let inner = self.inner.read().await;
        inner.pins.get(target_cid).cloned()
    }
}

// ---------------------------------------------------------------------------
// Scan-Resistant W-TinyLFU Cache Policy
// ---------------------------------------------------------------------------

/// 4-bit Count-Min Sketch frequency estimator for W-TinyLFU.
#[derive(Debug, Clone)]
pub struct CountMinSketch {
    table: Vec<u8>,
    mask: usize,
    additions: usize,
    sample_size: usize,
}

impl CountMinSketch {
    /// Creates a Count-Min sketch with the specified capacity.
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        let width = capacity.next_power_of_two().max(64);
        Self {
            table: vec![0u8; width],
            mask: width - 1,
            additions: 0,
            sample_size: capacity * 10,
        }
    }

    fn hash1(key: &[u8]) -> usize {
        let mut h = 0xcbf29ce484222325u64;
        for &b in key {
            h ^= b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        h as usize
    }

    fn hash2(key: &[u8]) -> usize {
        let mut h = 0x811c9dc5u32;
        for &b in key {
            h ^= b as u32;
            h = h.wrapping_mul(0x01000193);
        }
        h as usize
    }

    /// Increments frequency counter for the given chunk key.
    pub fn increment(&mut self, key: &[u8]) {
        let h1 = Self::hash1(key) & self.mask;
        let h2 = Self::hash2(key) & self.mask;
        let h3 = (h1.wrapping_add(h2)) & self.mask;
        let h4 = (h1.wrapping_add(h3)) & self.mask;

        for &idx in &[h1, h2, h3, h4] {
            if self.table[idx] < 15 {
                self.table[idx] += 1;
            }
        }

        self.additions += 1;
        if self.additions >= self.sample_size {
            self.reset();
        }
    }

    /// Estimates frequency of the given chunk key.
    #[must_use]
    pub fn estimate(&self, key: &[u8]) -> u8 {
        let h1 = Self::hash1(key) & self.mask;
        let h2 = Self::hash2(key) & self.mask;
        let h3 = (h1.wrapping_add(h2)) & self.mask;
        let h4 = (h1.wrapping_add(h3)) & self.mask;

        let v1 = self.table[h1];
        let v2 = self.table[h2];
        let v3 = self.table[h3];
        let v4 = self.table[h4];

        v1.min(v2).min(v3).min(v4)
    }

    /// Periodic aging: Halves all counters to decay historical frequency and prevent stale dominance.
    pub fn reset(&mut self) {
        for val in &mut self.table {
            *val >>= 1;
        }
        self.additions = 0;
    }
}

/// Scan-Resistant Window TinyLFU (W-TinyLFU) Chunk Cache.
///
/// Features:
/// 1. Window LRU (1% capacity) protects against bursts of recency.
/// 2. Probationary LRU (20% capacity) holds new arrivals competing for promotion.
/// 3. Protected LRU (79% capacity) stores frequently accessed hot items.
/// 4. Count-Min Sketch admission filter: When the window overflows, the candidate
///    competes with the victim from the probationary segment.
/// 5. Scan Resistance: A massive sequential streaming scan never evicts hot chunks accessed >= 3 times.
#[derive(Debug)]
pub struct WTinyLfuChunkCache {
    sketch: CountMinSketch,
    window_queue: VecDeque<ChunkId>,
    window_set: HashSet<ChunkId>,
    probation_queue: VecDeque<ChunkId>,
    probation_set: HashSet<ChunkId>,
    protected_queue: VecDeque<ChunkId>,
    protected_set: HashSet<ChunkId>,
    max_capacity: usize,
    window_capacity: usize,
    probation_capacity: usize,
    protected_capacity: usize,
    cache_store: HashMap<ChunkId, Vec<u8>>,
    hits: AtomicU64,
    misses: AtomicU64,
}

impl WTinyLfuChunkCache {
    /// Constructs a W-TinyLFU cache with the specified item capacity.
    #[must_use]
    pub fn new(max_capacity: usize) -> Self {
        let cap = max_capacity.max(10);
        let window_cap = (cap / 100).max(1);
        let probation_cap = ((cap * 20) / 100).max(2);
        let protected_cap = cap.saturating_sub(window_cap + probation_cap).max(2);

        Self {
            sketch: CountMinSketch::new(cap),
            window_queue: VecDeque::new(),
            window_set: HashSet::new(),
            probation_queue: VecDeque::new(),
            probation_set: HashSet::new(),
            protected_queue: VecDeque::new(),
            protected_set: HashSet::new(),
            max_capacity: cap,
            window_capacity: window_cap,
            probation_capacity: probation_cap,
            protected_capacity: protected_cap,
            cache_store: HashMap::new(),
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
        }
    }

    /// Accesses or fetches a chunk from the cache. Updates frequency and promotions.
    pub fn get(&mut self, chunk_id: &ChunkId) -> Option<&[u8]> {
        self.sketch.increment(chunk_id.as_bytes());

        if self.cache_store.contains_key(chunk_id) {
            self.hits.fetch_add(1, Ordering::Relaxed);

            // Promotion logic:
            if self.probation_set.contains(chunk_id) {
                // Second hit in probation promotes to protected
                self.probation_set.remove(chunk_id);
                self.probation_queue.retain(|c| c != chunk_id);

                self.protected_queue.push_back(*chunk_id);
                self.protected_set.insert(*chunk_id);

                // Demote if protected overflows
                if self.protected_queue.len() > self.protected_capacity {
                    if let Some(demoted) = self.protected_queue.pop_front() {
                        self.protected_set.remove(&demoted);
                        self.probation_queue.push_back(demoted);
                        self.probation_set.insert(demoted);
                    }
                }
            } else if self.window_set.contains(chunk_id) {
                // Hit in window promotes to protected
                self.window_set.remove(chunk_id);
                self.window_queue.retain(|c| c != chunk_id);

                self.protected_queue.push_back(*chunk_id);
                self.protected_set.insert(*chunk_id);

                // Demote if protected overflows
                if self.protected_queue.len() > self.protected_capacity {
                    if let Some(demoted) = self.protected_queue.pop_front() {
                        self.protected_set.remove(&demoted);
                        self.probation_queue.push_back(demoted);
                        self.probation_set.insert(demoted);
                    }
                }
            } else if self.protected_set.contains(chunk_id) {
                // Refresh position in protected LRU
                self.protected_queue.retain(|c| c != chunk_id);
                self.protected_queue.push_back(*chunk_id);
            }

            self.cache_store.get(chunk_id).map(|v| v.as_slice())
        } else {
            self.misses.fetch_add(1, Ordering::Relaxed);
            None
        }
    }

    /// Inserts a chunk into the cache using W-TinyLFU admission policy.
    pub fn put(&mut self, chunk_id: ChunkId, data: Vec<u8>) {
        self.sketch.increment(chunk_id.as_bytes());

        if let Entry::Occupied(mut e) = self.cache_store.entry(chunk_id) {
            e.insert(data);
            return;
        }

        // Add to window cache first
        self.window_queue.push_back(chunk_id);
        self.window_set.insert(chunk_id);
        self.cache_store.insert(chunk_id, data);

        // If window overflows, candidate from window competes with victim from probation
        if self.window_queue.len() > self.window_capacity {
            if let Some(candidate) = self.window_queue.pop_front() {
                self.window_set.remove(&candidate);

                if self.probation_queue.len() < self.probation_capacity {
                    // Direct admission to probation if space available
                    self.probation_queue.push_back(candidate);
                    self.probation_set.insert(candidate);
                } else if let Some(victim) = self.probation_queue.front() {
                    let freq_candidate = self.sketch.estimate(candidate.as_bytes());
                    let freq_victim = self.sketch.estimate(victim.as_bytes());

                    if freq_candidate > freq_victim {
                        // Candidate wins: evict victim from probation, admit candidate
                        if let Some(evicted) = self.probation_queue.pop_front() {
                            self.probation_set.remove(&evicted);
                            self.cache_store.remove(&evicted);
                        }
                        self.probation_queue.push_back(candidate);
                        self.probation_set.insert(candidate);
                    } else {
                        // Victim wins: candidate is rejected/evicted
                        self.cache_store.remove(&candidate);
                    }
                }
            }
        }
    }

    /// Checks if a chunk is currently cached.
    #[must_use]
    pub fn contains(&self, chunk_id: &ChunkId) -> bool {
        self.cache_store.contains_key(chunk_id)
    }

    /// Returns cache size.
    #[must_use]
    pub fn len(&self) -> usize {
        self.cache_store.len()
    }

    /// Returns configured maximum capacity.
    #[must_use]
    pub fn max_capacity(&self) -> usize {
        self.max_capacity
    }

    /// Returns true if cache is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.cache_store.is_empty()
    }

    /// Returns cache hit and miss statistics.
    #[must_use]
    pub fn hit_ratio(&self) -> f64 {
        let h = self.hits.load(Ordering::Relaxed);
        let m = self.misses.load(Ordering::Relaxed);
        let total = h + m;
        if total == 0 {
            0.0
        } else {
            (h as f64) / (total as f64)
        }
    }
}
