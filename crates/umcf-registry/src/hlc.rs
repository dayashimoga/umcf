//! Monotonic Hybrid Logical Clocks (HLC) with bounded clock drift enforcement.
//! Strictly enforces `#![forbid(unsafe_code)]` and zero-placeholder invariants.

use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;

/// Default maximum allowed clock drift between nodes before rejection (5,000 ms = 5 seconds).
pub const DEFAULT_MAX_DRIFT_MS: u64 = 5_000;

/// Monotonic, globally ordered timestamp combining physical wall-clock time,
/// a logical disambiguation counter, and a deterministic node hash tie-breaker.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct HlcTimestamp {
    /// Physical wall time in milliseconds since UNIX epoch.
    pub wall_ms: u64,
    /// Logical counter for sub-millisecond causal events.
    pub logical: u32,
    /// Truncated hash of the issuing node ID as a deterministic tie-breaker.
    pub node_hash: u32,
}

impl HlcTimestamp {
    /// Zero/minimum timestamp.
    pub const MIN: Self = Self {
        wall_ms: 0,
        logical: 0,
        node_hash: 0,
    };

    /// Maximum possible timestamp.
    pub const MAX: Self = Self {
        wall_ms: u64::MAX,
        logical: u32::MAX,
        node_hash: u32::MAX,
    };

    /// Constructs a new explicit `HlcTimestamp`.
    #[must_use]
    pub const fn new(wall_ms: u64, logical: u32, node_hash: u32) -> Self {
        Self {
            wall_ms,
            logical,
            node_hash,
        }
    }

    /// Derives a deterministic `HlcTimestamp` with a node hash from `node_id`.
    #[must_use]
    pub fn from_node_id(node_id: &str, wall_ms: u64, logical: u32) -> Self {
        let hash_bytes = blake3::hash(node_id.as_bytes());
        let hash = u32::from_le_bytes([
            hash_bytes.as_bytes()[0],
            hash_bytes.as_bytes()[1],
            hash_bytes.as_bytes()[2],
            hash_bytes.as_bytes()[3],
        ]);
        Self {
            wall_ms,
            logical,
            node_hash: hash,
        }
    }
}

impl Ord for HlcTimestamp {
    fn cmp(&self, other: &Self) -> Ordering {
        self.wall_ms
            .cmp(&other.wall_ms)
            .then_with(|| self.logical.cmp(&other.logical))
            .then_with(|| self.node_hash.cmp(&other.node_hash))
    }
}

impl PartialOrd for HlcTimestamp {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Hybrid Logical Clock diagnostic errors.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum HlcError {
    #[error("Clock drift exceeded limit: remote wall {remote_wall_ms} ms exceeds local {local_wall_ms} ms by more than max drift {max_drift_ms} ms")]
    ClockDriftExceeded {
        remote_wall_ms: u64,
        local_wall_ms: u64,
        max_drift_ms: u64,
    },

    #[error("Logical counter overflowed u32 at wall_ms {wall_ms}")]
    LogicalOverflow { wall_ms: u64 },
}

/// Thread-safe Monotonic Hybrid Logical Clock generator.
#[derive(Debug, Clone)]
pub struct HybridLogicalClock {
    node_hash: u32,
    max_drift_ms: u64,
    state: Arc<Mutex<HlcTimestamp>>,
}

impl HybridLogicalClock {
    /// Creates a new `HybridLogicalClock` with default 5-second drift ceiling.
    #[must_use]
    pub fn new(node_hash: u32) -> Self {
        Self::with_max_drift(node_hash, DEFAULT_MAX_DRIFT_MS)
    }

    /// Creates a new `HybridLogicalClock` configured with custom drift ceiling.
    #[must_use]
    pub fn with_max_drift(node_hash: u32, max_drift_ms: u64) -> Self {
        let now = Self::system_time_ms();
        Self {
            node_hash,
            max_drift_ms,
            state: Arc::new(Mutex::new(HlcTimestamp::new(now, 0, node_hash))),
        }
    }

    /// Creates a new `HybridLogicalClock` deriving `node_hash` from a node DID/string.
    #[must_use]
    pub fn from_node_id(node_id: &str) -> Self {
        let hash_bytes = blake3::hash(node_id.as_bytes());
        let hash = u32::from_le_bytes([
            hash_bytes.as_bytes()[0],
            hash_bytes.as_bytes()[1],
            hash_bytes.as_bytes()[2],
            hash_bytes.as_bytes()[3],
        ]);
        Self::new(hash)
    }

    /// Creates a clock initialized with a specific timestamp (e.g. from storage or test setup).
    #[must_use]
    pub fn with_initial_timestamp(initial: HlcTimestamp, max_drift_ms: u64) -> Self {
        Self {
            node_hash: initial.node_hash,
            max_drift_ms,
            state: Arc::new(Mutex::new(initial)),
        }
    }

    /// Returns the current latest timestamp snapshot without advancing it.
    #[must_use]
    pub fn now(&self) -> HlcTimestamp {
        *self.state.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Updates internal state to ensure it is at least `ts`.
    pub fn update_floor(&self, ts: HlcTimestamp) {
        let mut cur = self.state.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        if ts > *cur {
            *cur = ts;
            cur.node_hash = self.node_hash;
        }
    }

    /// Advances the clock for a local event using system physical time.
    pub fn tick(&self) -> Result<HlcTimestamp, HlcError> {
        self.tick_with_physical_time(Self::system_time_ms())
    }

    /// Advances the clock for a local event with an explicit physical clock (for deterministic testing).
    pub fn tick_with_physical_time(&self, phys_now_ms: u64) -> Result<HlcTimestamp, HlcError> {
        let mut cur = self.state.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        if phys_now_ms > cur.wall_ms {
            cur.wall_ms = phys_now_ms;
            cur.logical = 0;
        } else {
            cur.logical = cur
                .logical
                .checked_add(1)
                .ok_or(HlcError::LogicalOverflow { wall_ms: cur.wall_ms })?;
        }
        cur.node_hash = self.node_hash;
        Ok(*cur)
    }

    /// Observes a remote timestamp, enforcing clock-drift bounds and causal monotonicity.
    pub fn observe(&self, remote: &HlcTimestamp) -> Result<HlcTimestamp, HlcError> {
        self.observe_with_physical_time(remote, Self::system_time_ms())
    }

    /// Observes a remote timestamp with an explicit physical clock (for deterministic testing).
    pub fn observe_with_physical_time(
        &self,
        remote: &HlcTimestamp,
        phys_now_ms: u64,
    ) -> Result<HlcTimestamp, HlcError> {
        // Enforce clock drift bound: remote wall must not be ahead of local physical clock by more than max_drift_ms
        if remote.wall_ms > phys_now_ms.saturating_add(self.max_drift_ms) {
            return Err(HlcError::ClockDriftExceeded {
                remote_wall_ms: remote.wall_ms,
                local_wall_ms: phys_now_ms,
                max_drift_ms: self.max_drift_ms,
            });
        }

        let mut cur = self.state.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let max_wall = cur.wall_ms.max(remote.wall_ms).max(phys_now_ms);

        if max_wall == cur.wall_ms && max_wall == remote.wall_ms {
            cur.logical = cur
                .logical
                .max(remote.logical)
                .checked_add(1)
                .ok_or(HlcError::LogicalOverflow { wall_ms: max_wall })?;
        } else if max_wall == cur.wall_ms {
            cur.logical = cur
                .logical
                .checked_add(1)
                .ok_or(HlcError::LogicalOverflow { wall_ms: max_wall })?;
        } else if max_wall == remote.wall_ms {
            cur.wall_ms = max_wall;
            cur.logical = remote
                .logical
                .checked_add(1)
                .ok_or(HlcError::LogicalOverflow { wall_ms: max_wall })?;
        } else {
            cur.wall_ms = max_wall;
            cur.logical = 0;
        }
        cur.node_hash = self.node_hash;
        Ok(*cur)
    }

    /// Returns current system UNIX timestamp in milliseconds.
    #[must_use]
    pub fn system_time_ms() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }

    /// Returns the node hash configured for this clock.
    #[must_use]
    pub fn node_hash(&self) -> u32 {
        self.node_hash
    }

    /// Returns the maximum allowed clock drift in milliseconds.
    #[must_use]
    pub fn max_drift_ms(&self) -> u64 {
        self.max_drift_ms
    }
}
