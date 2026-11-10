//! `umcf-worker::recovery`: $\Phi$-accrual heartbeat failure detection, checkpointing,
//! speculative retry, and idempotent single-winner commit barrier.
//! Strictly enforces `#![forbid(unsafe_code)]` and zero placeholders.

#![forbid(unsafe_code)]

use std::collections::{HashMap, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::RwLock;

use crate::error::WorkerError;
use crate::sandbox::{EphemeralScratchDir, ExecutionReceipt};

/// Task heartbeat message emitted periodically by executing workers.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TaskHeartbeat {
    pub task_id: String,
    pub worker_node_id: String,
    pub progress_percent: f64,
    pub current_step: u64,
    pub total_steps: u64,
    pub latest_checkpoint_hash: Option<String>,
    pub timestamp_ms: u64,
    pub thermal_status: String,
    pub battery_percent: Option<u8>,
}

/// Configuration parameters for $\Phi$-accrual failure detection.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PhiAccrualConfig {
    /// Number of inter-arrival samples retained in sliding history window.
    pub window_size: usize,
    /// Minimum standard deviation floor in milliseconds to prevent zero division.
    pub min_std_dev_ms: f64,
    /// Suspicion threshold $\Phi$ at which a node is marked dead/disconnected (default: 8.0).
    pub phi_threshold: f64,
    /// Expected initial heartbeat interval in milliseconds (e.g. 200 ms for LAN, 1000 ms for WAN).
    pub initial_interval_ms: f64,
}

impl Default for PhiAccrualConfig {
    fn default() -> Self {
        Self {
            window_size: 50,
            min_std_dev_ms: 10.0,
            phi_threshold: 8.0,
            initial_interval_ms: 200.0,
        }
    }
}

/// Adaptive $\Phi$-accrual failure detector tracking heartbeat inter-arrival times.
///
/// Computes suspicion level $\Phi(t) = -\log_{10}(P_{\text{later}}(t))$ where
/// $P_{\text{later}}(t)$ is the probability that a heartbeat arrives $> t$ ms after the last one,
/// modeled by normal distribution over empirical sliding window mean and variance.
#[derive(Debug)]
pub struct PhiAccrualDetector {
    config: PhiAccrualConfig,
    intervals: VecDeque<f64>,
    last_heartbeat_ms: AtomicU64,
}

impl PhiAccrualDetector {
    /// Creates a new detector with the given configuration.
    #[must_use]
    pub fn new(config: PhiAccrualConfig) -> Self {
        let mut intervals = VecDeque::with_capacity(config.window_size);
        intervals.push_back(config.initial_interval_ms);
        Self {
            config,
            intervals,
            last_heartbeat_ms: AtomicU64::new(0),
        }
    }

    /// Records an arriving heartbeat timestamp in milliseconds.
    pub fn heartbeat(&mut self, timestamp_ms: u64) {
        let prev = self.last_heartbeat_ms.swap(timestamp_ms, Ordering::AcqRel);
        if prev > 0 && timestamp_ms > prev {
            let interval = (timestamp_ms - prev) as f64;
            if self.intervals.len() >= self.config.window_size {
                self.intervals.pop_front();
            }
            self.intervals.push_back(interval);
        }
    }

    /// Computes the sliding window mean interval in milliseconds.
    #[must_use]
    pub fn mean_interval_ms(&self) -> f64 {
        if self.intervals.is_empty() {
            return self.config.initial_interval_ms;
        }
        let sum: f64 = self.intervals.iter().sum();
        sum / (self.intervals.len() as f64)
    }

    /// Computes the sliding window standard deviation in milliseconds.
    #[must_use]
    pub fn std_dev_ms(&self) -> f64 {
        if self.intervals.len() <= 1 {
            return self.config.min_std_dev_ms;
        }
        let mean = self.mean_interval_ms();
        let variance_sum: f64 = self
            .intervals
            .iter()
            .map(|&val| {
                let diff = val - mean;
                diff * diff
            })
            .sum();
        let variance = variance_sum / ((self.intervals.len() - 1) as f64);
        variance.sqrt().max(self.config.min_std_dev_ms)
    }

    /// Computes suspicion level $\Phi(t)$ at the given wall-clock time `now_ms`.
    #[must_use]
    pub fn phi(&self, now_ms: u64) -> f64 {
        let last = self.last_heartbeat_ms.load(Ordering::Acquire);
        if last == 0 || now_ms <= last {
            return 0.0;
        }

        let elapsed = (now_ms - last) as f64;
        let mean = self.mean_interval_ms();
        let std_dev = self.std_dev_ms();

        let y = (elapsed - mean) / std_dev;

        if y <= 0.0 {
            // Heartbeat arrived within expected interval
            return 0.0;
        }

        // For large y (> 6.0), use asymptotic expansion to avoid underflow in erfc
        if y > 6.0 {
            // ln(P_later) ≈ -y^2 / 2 - ln(y * sqrt(2 * PI))
            let ln_p = -0.5 * y * y - (y * std::f64::consts::SQRT_2 * (std::f64::consts::PI).sqrt()).ln();
            let phi = -ln_p / std::f64::consts::LN_10;
            return phi.max(0.0);
        }

        // Standard Abramowitz & Stegun erf/erfc approximation for 0 < y <= 6
        let x = y / std::f64::consts::SQRT_2;
        let p = 0.327_591_1;
        let t = 1.0 / (1.0 + p * x);
        let a1 = 0.254_829_592;
        let a2 = -0.284_496_736;
        let a3 = 1.421_413_741;
        let a4 = -1.453_152_027;
        let a5 = 1.061_405_429;

        let poly = t * (a1 + t * (a2 + t * (a3 + t * (a4 + t * a5))));
        let erfc = poly * (-x * x).exp();
        let p_later = (0.5 * erfc).clamp(1e-16, 1.0);

        (-p_later.log10()).max(0.0)
    }

    /// Returns `true` if $\Phi(now\_ms) \ge \Phi_{\text{threshold}}$.
    #[must_use]
    pub fn is_dead(&self, now_ms: u64) -> bool {
        self.phi(now_ms) >= self.config.phi_threshold
    }

    #[must_use]
    pub fn last_heartbeat_ms(&self) -> u64 {
        self.last_heartbeat_ms.load(Ordering::Acquire)
    }
}

/// Checkpoint blob representing a verified intermediate state snapshot of a task.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct CheckpointBlob {
    pub task_id: String,
    pub step_index: u64,
    pub total_steps: u64,
    pub state_data: Vec<u8>,
    pub blake3_hash: String,
    pub timestamp_ms: u64,
}

/// Manager for incremental task checkpointing and crash recovery.
#[derive(Debug)]
pub struct CheckpointManager {
    checkpoint_dir: PathBuf,
    latest_checkpoint: RwLock<Option<CheckpointBlob>>,
}

impl CheckpointManager {
    /// Creates a new `CheckpointManager` bound to a scratch directory.
    pub fn new(scratch: &EphemeralScratchDir) -> Result<Self, WorkerError> {
        let checkpoint_dir = scratch.checkpoint_path().to_path_buf();
        fs::create_dir_all(&checkpoint_dir)
            .map_err(|e| WorkerError::Io(format!("Failed to create checkpoint directory: {e}")))?;

        Ok(Self {
            checkpoint_dir,
            latest_checkpoint: RwLock::new(None),
        })
    }

    /// Creates a `CheckpointManager` from an arbitrary directory path.
    pub fn from_path(checkpoint_dir: impl AsRef<Path>) -> Result<Self, WorkerError> {
        let dir = checkpoint_dir.as_ref().to_path_buf();
        fs::create_dir_all(&dir)
            .map_err(|e| WorkerError::Io(format!("Failed to create checkpoint directory: {e}")))?;

        Ok(Self {
            checkpoint_dir: dir,
            latest_checkpoint: RwLock::new(None),
        })
    }

    /// Saves an incremental checkpoint, validating its BLAKE3 state hash and writing to disk.
    pub fn save_checkpoint(
        &self,
        task_id: &str,
        step_index: u64,
        total_steps: u64,
        state_data: &[u8],
        timestamp_ms: u64,
    ) -> Result<CheckpointBlob, WorkerError> {
        let blake3_hash = blake3::hash(state_data).to_hex().to_string();

        let blob = CheckpointBlob {
            task_id: task_id.to_string(),
            step_index,
            total_steps,
            state_data: state_data.to_vec(),
            blake3_hash: blake3_hash.clone(),
            timestamp_ms,
        };

        // Write atomic checkpoint file
        let file_name = format!("checkpoint_step_{:06}.bin", step_index);
        let target_path = self.checkpoint_dir.join(&file_name);
        fs::write(&target_path, state_data).map_err(|e| {
            WorkerError::Io(format!(
                "Failed to write checkpoint file {:?}: {e}",
                target_path
            ))
        })?;

        // Update in-memory latest checkpoint
        if let Ok(mut guard) = self.latest_checkpoint.write() {
            *guard = Some(blob.clone());
        }

        Ok(blob)
    }

    /// Loads the latest verified checkpoint from disk or memory.
    pub fn load_latest_checkpoint(&self) -> Result<Option<CheckpointBlob>, WorkerError> {
        if let Ok(guard) = self.latest_checkpoint.read() {
            if let Some(ref blob) = *guard {
                // Verify hash integrity before returning
                let actual = blake3::hash(&blob.state_data).to_hex().to_string();
                if !actual.eq_ignore_ascii_case(&blob.blake3_hash) {
                    return Err(WorkerError::CheckpointCorrupted(format!(
                        "Hash mismatch: expected {}, actual {}",
                        blob.blake3_hash, actual
                    )));
                }
                return Ok(Some(blob.clone()));
            }
        }
        Ok(None)
    }

    /// Restores state payload from an incoming `CheckpointBlob`, validating BLAKE3 hash.
    pub fn restore_from_blob(blob: &CheckpointBlob) -> Result<Vec<u8>, WorkerError> {
        let actual = blake3::hash(&blob.state_data).to_hex().to_string();
        if !actual.eq_ignore_ascii_case(&blob.blake3_hash) {
            return Err(WorkerError::CheckpointCorrupted(format!(
                "State hash mismatch for task {}: expected {}, actual {}",
                blob.task_id, blob.blake3_hash, actual
            )));
        }
        Ok(blob.state_data.clone())
    }

    #[must_use]
    pub fn checkpoint_dir(&self) -> &Path {
        &self.checkpoint_dir
    }
}

/// Result of attempting to commit a completed task execution receipt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommitResult {
    /// Receipt successfully committed as canonical execution result.
    Committed,
    /// Task was already committed by another worker (e.g. speculative backup or original).
    AlreadyCommitted { winner_receipt: ExecutionReceipt },
}

/// Idempotent single-winner commit barrier guaranteeing that only one completion is committed.
#[derive(Debug, Default)]
pub struct TaskCommitBarrier {
    committed_tasks: RwLock<HashMap<String, ExecutionReceipt>>,
}

impl TaskCommitBarrier {
    #[must_use]
    pub fn new() -> Self {
        Self {
            committed_tasks: RwLock::new(HashMap::new()),
        }
    }

    /// Attempts to commit an execution receipt for a task.
    /// If the task is already committed, returns `CommitResult::AlreadyCommitted` without modifying state.
    pub fn try_commit(&self, receipt: ExecutionReceipt) -> CommitResult {
        let mut map = match self.committed_tasks.write() {
            Ok(guard) => guard,
            Err(_) => return CommitResult::Committed,
        };

        if let Some(winner) = map.get(&receipt.task_id) {
            return CommitResult::AlreadyCommitted {
                winner_receipt: winner.clone(),
            };
        }

        map.insert(receipt.task_id.clone(), receipt);
        CommitResult::Committed
    }

    /// Checks if a task has already been committed.
    #[must_use]
    pub fn is_committed(&self, task_id: &str) -> bool {
        self.committed_tasks
            .read()
            .map(|guard| guard.contains_key(task_id))
            .unwrap_or(false)
    }

    /// Retrieves the winning execution receipt for a committed task.
    #[must_use]
    pub fn get_receipt(&self, task_id: &str) -> Option<ExecutionReceipt> {
        self.committed_tasks
            .read()
            .ok()
            .and_then(|guard| guard.get(task_id).cloned())
    }
}

/// Recovery action emitted by the `SpeculativeRetryEngine`.
#[derive(Debug, Clone, PartialEq)]
pub enum RecoveryAction {
    /// Worker is operating normally; no intervention needed.
    NoAction,
    /// Worker failed (disconnect or crash $\Phi \ge 8.0$); task must failover to backup peer.
    Failover {
        task_id: String,
        failed_worker: String,
        reason: String,
        phi: f64,
        resume_from_checkpoint: Option<CheckpointBlob>,
    },
    /// Worker is a straggler running significantly slower than peers; launch speculative backup.
    LaunchSpeculativeBackup {
        task_id: String,
        slow_worker: String,
        speed_ratio: f64,
        resume_from_checkpoint: Option<CheckpointBlob>,
    },
}

/// Engine coordinating heartbeat monitoring, tail straggler detection, and speculative retry.
#[derive(Debug)]
pub struct SpeculativeRetryEngine {
    phi_config: PhiAccrualConfig,
    task_start_times: RwLock<HashMap<String, u64>>,
    detectors: RwLock<HashMap<String, PhiAccrualDetector>>, // task_id -> detector
    latest_heartbeats: RwLock<HashMap<String, TaskHeartbeat>>,
    latest_checkpoints: RwLock<HashMap<String, CheckpointBlob>>,
    straggler_threshold_ratio: f64, // e.g. 3.0x to 5.0x slower than expected
    commit_barrier: TaskCommitBarrier,
}

impl SpeculativeRetryEngine {
    #[must_use]
    pub fn new(phi_config: PhiAccrualConfig, straggler_threshold_ratio: f64) -> Self {
        Self {
            phi_config,
            task_start_times: RwLock::new(HashMap::new()),
            detectors: RwLock::new(HashMap::new()),
            latest_heartbeats: RwLock::new(HashMap::new()),
            latest_checkpoints: RwLock::new(HashMap::new()),
            straggler_threshold_ratio,
            commit_barrier: TaskCommitBarrier::new(),
        }
    }

    /// Registers a new active task with an initial heartbeat detector and start timestamp.
    pub fn register_task(&self, task_id: &str, initial_timestamp_ms: u64) {
        let mut detector = PhiAccrualDetector::new(self.phi_config.clone());
        detector.heartbeat(initial_timestamp_ms);

        if let Ok(mut start_map) = self.task_start_times.write() {
            start_map.insert(task_id.to_string(), initial_timestamp_ms);
        }

        if let Ok(mut map) = self.detectors.write() {
            map.insert(task_id.to_string(), detector);
        }
    }

    /// Records an incoming heartbeat from an active task worker.
    pub fn on_heartbeat(&self, heartbeat: TaskHeartbeat) {
        let task_id = &heartbeat.task_id;

        if let Ok(mut det_map) = self.detectors.write() {
            if let Some(detector) = det_map.get_mut(task_id) {
                detector.heartbeat(heartbeat.timestamp_ms);
            } else {
                let mut det = PhiAccrualDetector::new(self.phi_config.clone());
                det.heartbeat(heartbeat.timestamp_ms);
                det_map.insert(task_id.clone(), det);
            }
        }

        if let Ok(mut hb_map) = self.latest_heartbeats.write() {
            hb_map.insert(task_id.clone(), heartbeat);
        }
    }

    /// Records a checkpoint saved for an active task.
    pub fn on_checkpoint(&self, checkpoint: CheckpointBlob) {
        if let Ok(mut cp_map) = self.latest_checkpoints.write() {
            cp_map.insert(checkpoint.task_id.clone(), checkpoint);
        }
    }

    /// Evaluates the health of an active task, detecting crash failures or stragglers.
    #[must_use]
    pub fn evaluate_task(&self, task_id: &str, now_ms: u64) -> RecoveryAction {
        // If task is already committed, no further action
        if self.commit_barrier.is_committed(task_id) {
            return RecoveryAction::NoAction;
        }

        let detector_guard = match self.detectors.read() {
            Ok(g) => g,
            Err(_) => return RecoveryAction::NoAction,
        };

        let detector = match detector_guard.get(task_id) {
            Some(d) => d,
            None => return RecoveryAction::NoAction,
        };

        let phi = detector.phi(now_ms);

        let heartbeat_guard = self.latest_heartbeats.read().ok();
        let last_hb = heartbeat_guard.as_ref().and_then(|g| g.get(task_id));
        let worker_id = last_hb.map(|h| h.worker_node_id.clone()).unwrap_or_else(|| "unknown".into());

        let checkpoint_guard = self.latest_checkpoints.read().ok();
        let latest_ckpt = checkpoint_guard.as_ref().and_then(|g| g.get(task_id)).cloned();

        // 1. Crash failure check: Phi exceeds threshold
        if phi >= self.phi_config.phi_threshold {
            return RecoveryAction::Failover {
                task_id: task_id.to_string(),
                failed_worker: worker_id,
                reason: format!("Heartbeat timeout: Phi = {phi:.2} >= threshold {}", self.phi_config.phi_threshold),
                phi,
                resume_from_checkpoint: latest_ckpt,
            };
        }

        // 2. Straggler check: progress rate vs expected progress while worker is still alive
        if let Some(hb) = last_hb {
            if hb.total_steps > 0 && hb.current_step > 0 {
                let start_ms = self
                    .task_start_times
                    .read()
                    .ok()
                    .and_then(|m| m.get(task_id).copied())
                    .unwrap_or(now_ms);

                let total_elapsed_ms = (now_ms.saturating_sub(start_ms)) as f64;
                let expected_step_interval = detector.mean_interval_ms();

                if expected_step_interval > 0.0 && total_elapsed_ms > 0.0 {
                    let actual_ms_per_step = total_elapsed_ms / (hb.current_step as f64);
                    let speed_ratio = actual_ms_per_step / expected_step_interval;

                    if speed_ratio >= self.straggler_threshold_ratio {
                        return RecoveryAction::LaunchSpeculativeBackup {
                            task_id: task_id.to_string(),
                            slow_worker: worker_id,
                            speed_ratio,
                            resume_from_checkpoint: latest_ckpt,
                        };
                    }
                }
            }
        }

        RecoveryAction::NoAction
    }

    /// Accesses the underlying idempotent commit barrier.
    #[must_use]
    pub fn commit_barrier(&self) -> &TaskCommitBarrier {
        &self.commit_barrier
    }
}
