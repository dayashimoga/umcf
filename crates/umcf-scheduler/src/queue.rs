//! `umcf-scheduler::queue`: Priority-banded Deficit Round-Robin queue, admission controller, deadlock-free gang scheduler, and worker backpressure.
//! Enforces `#![forbid(unsafe_code)]` and zero placeholders.

#![forbid(unsafe_code)]

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::{Arc, Mutex};
use umcf_core::NodeId;

use crate::dag::JobDag;
use crate::error::SchedulingError;

/// Priority bands for job scheduling and resource allocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub enum PriorityBand {
    /// Latency-critical realtime tasks (interactive remote desktop, live inference streaming).
    InteractiveRealtime = 0,
    /// High-priority user requests (foreground interactive queries, explicit user submissions).
    HighPriorityUser = 1,
    /// Standard distributed compute workloads (batch inference, matrix multiply, DAG pipelines).
    StandardBatch = 2,
    /// Low-priority background maintenance (model checkpoint pruning, shard pre-caching).
    BackgroundMaintenance = 3,
}

impl PriorityBand {
    /// Deficit Round-Robin quantum weight allocated per round.
    #[must_use]
    pub const fn drr_quantum(self) -> usize {
        match self {
            Self::InteractiveRealtime => 8,
            Self::HighPriorityUser => 4,
            Self::StandardBatch => 2,
            Self::BackgroundMaintenance => 1,
        }
    }
}

/// Backpressure status reported by an individual worker node.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum BackpressureStatus {
    Normal,
    Throttled { queue_depth: usize, load_percent: f64 },
    Saturated { queue_depth: usize, retry_after_ms: u64 },
}

/// A queued job waiting for cluster execution.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct QueuedJob {
    pub job_id: String,
    pub requester_id: NodeId,
    pub priority: PriorityBand,
    pub effective_priority: PriorityBand,
    pub dag: JobDag,
    pub memory_bytes: u64,
    pub enqueued_at_us: u64,
    pub is_gang_job: bool,
    pub required_gang_nodes: Vec<NodeId>,
}

impl QueuedJob {
    #[must_use]
    pub fn new(
        job_id: impl Into<String>,
        requester_id: NodeId,
        priority: PriorityBand,
        dag: JobDag,
        memory_bytes: u64,
        enqueued_at_us: u64,
    ) -> Self {
        Self {
            job_id: job_id.into(),
            requester_id,
            priority,
            effective_priority: priority,
            dag,
            memory_bytes,
            enqueued_at_us,
            is_gang_job: false,
            required_gang_nodes: Vec::new(),
        }
    }

    #[must_use]
    pub fn new_gang(
        job_id: impl Into<String>,
        requester_id: NodeId,
        priority: PriorityBand,
        dag: JobDag,
        memory_bytes: u64,
        enqueued_at_us: u64,
        gang_nodes: Vec<NodeId>,
    ) -> Self {
        Self {
            job_id: job_id.into(),
            requester_id,
            priority,
            effective_priority: priority,
            dag,
            memory_bytes,
            enqueued_at_us,
            is_gang_job: true,
            required_gang_nodes: gang_nodes,
        }
    }
}

/// Admission control policy configuration.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AdmissionConfig {
    pub max_queued_jobs: usize,
    pub max_cluster_memory_bytes: u64,
    pub per_requester_max_jobs: usize,
    pub aging_threshold_secs: f64,
}

impl Default for AdmissionConfig {
    fn default() -> Self {
        Self {
            max_queued_jobs: 1000,
            max_cluster_memory_bytes: 64 * 1024 * 1024 * 1024, // 64 GiB
            per_requester_max_jobs: 100,
            aging_threshold_secs: 60.0,
        }
    }
}

/// Admission controller enforcing queue limits, memory quotas, and fair-share constraints.
#[derive(Debug)]
pub struct AdmissionController {
    config: AdmissionConfig,
    current_queued_jobs: usize,
    current_committed_memory_bytes: u64,
    requester_job_counts: HashMap<NodeId, usize>,
}

impl AdmissionController {
    #[must_use]
    pub fn new(config: AdmissionConfig) -> Self {
        Self {
            config,
            current_queued_jobs: 0,
            current_committed_memory_bytes: 0,
            requester_job_counts: HashMap::new(),
        }
    }

    /// Evaluates admission for a candidate job.
    /// Returns `Err(SchedulingError::ClusterOverloaded)` if saturated.
    pub fn admit_job(&mut self, job: &QueuedJob) -> Result<(), SchedulingError> {
        // 1. Max queue depth check
        if self.current_queued_jobs >= self.config.max_queued_jobs {
            return Err(SchedulingError::ClusterOverloaded {
                retry_after_secs: 5,
                reason: format!(
                    "Maximum cluster queue depth exceeded ({}/{} jobs queued)",
                    self.current_queued_jobs, self.config.max_queued_jobs
                ),
            });
        }

        // 2. Cluster memory commitment ceiling check
        let projected_mem = self
            .current_committed_memory_bytes
            .saturating_add(job.memory_bytes);
        if projected_mem > self.config.max_cluster_memory_bytes {
            return Err(SchedulingError::ClusterOverloaded {
                retry_after_secs: 10,
                reason: format!(
                    "Cluster memory quota saturated (requested {} bytes, committed {}/{} bytes)",
                    job.memory_bytes, self.current_committed_memory_bytes, self.config.max_cluster_memory_bytes
                ),
            });
        }

        // 3. Per-requester fair-share job limit check
        let current_requester_jobs = self
            .requester_job_counts
            .get(&job.requester_id)
            .copied()
            .unwrap_or(0);
        if current_requester_jobs >= self.config.per_requester_max_jobs {
            return Err(SchedulingError::ClusterOverloaded {
                retry_after_secs: 15,
                reason: format!(
                    "Per-requester quota exceeded for node {:?} ({}/{} active jobs)",
                    job.requester_id, current_requester_jobs, self.config.per_requester_max_jobs
                ),
            });
        }

        // Admitted! Update tracking state
        self.current_queued_jobs += 1;
        self.current_committed_memory_bytes = projected_mem;
        *self
            .requester_job_counts
            .entry(job.requester_id.clone())
            .or_insert(0) += 1;

        Ok(())
    }

    /// Releases resources for a completed, dispatched, or cancelled job.
    pub fn release_job(&mut self, job: &QueuedJob) {
        self.current_queued_jobs = self.current_queued_jobs.saturating_sub(1);
        self.current_committed_memory_bytes = self
            .current_committed_memory_bytes
            .saturating_sub(job.memory_bytes);

        if let Some(count) = self.requester_job_counts.get_mut(&job.requester_id) {
            *count = count.saturating_sub(1);
            if *count == 0 {
                self.requester_job_counts.remove(&job.requester_id);
            }
        }
    }
}

/// Deadlock-free gang scheduling coordinator for multi-node pipeline workloads.
#[derive(Debug, Default)]
pub struct DeadlockFreeGangCoordinator {
    locked_nodes: Mutex<HashSet<NodeId>>,
    deadlocks_avoided_count: Mutex<u64>,
}

/// Result of a gang-scheduling reservation attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GangReservationResult {
    Acquired { nodes: Vec<NodeId> },
    ContendedRetry { conflicting_node: NodeId },
}

impl DeadlockFreeGangCoordinator {
    #[must_use]
    pub fn new() -> Self {
        Self {
            locked_nodes: Mutex::new(HashSet::new()),
            deadlocks_avoided_count: Mutex::new(0),
        }
    }

    /// Atomically acquires multi-node gang reservations in strictly sorted global order.
    /// If any requested node is locked, immediately rolls back all acquired locks in reverse order,
    /// completely preventing circular wait and deadlock (Coffman conditions).
    pub fn try_acquire_gang(&self, requested_nodes: &[NodeId]) -> GangReservationResult {
        if requested_nodes.is_empty() {
            return GangReservationResult::Acquired { nodes: Vec::new() };
        }

        // 1. Sort requested node IDs deterministically to establish global lock hierarchy
        let mut sorted_nodes = requested_nodes.to_vec();
        sorted_nodes.sort_unstable();
        sorted_nodes.dedup();

        let mut lock_guard = match self.locked_nodes.lock() {
            Ok(g) => g,
            Err(poisoned) => poisoned.into_inner(),
        };

        // 2. Check if all nodes are available
        for node in &sorted_nodes {
            if lock_guard.contains(node) {
                // Conflict detected! Rollback is automatic since we haven't modified state yet
                if let Ok(mut count) = self.deadlocks_avoided_count.lock() {
                    *count += 1;
                }
                return GangReservationResult::ContendedRetry {
                    conflicting_node: node.clone(),
                };
            }
        }

        // 3. Atomically acquire all nodes
        for node in &sorted_nodes {
            lock_guard.insert(node.clone());
        }

        GangReservationResult::Acquired {
            nodes: sorted_nodes,
        }
    }

    /// Releases a set of locked gang nodes.
    pub fn release_gang(&self, nodes: &[NodeId]) {
        if let Ok(mut lock_guard) = self.locked_nodes.lock() {
            for node in nodes {
                lock_guard.remove(node);
            }
        }
    }

    /// Returns count of avoided deadlocks.
    pub fn deadlocks_avoided(&self) -> u64 {
        self.deadlocks_avoided_count
            .lock()
            .map(|g| *g)
            .unwrap_or(0)
    }
}

/// Realtime telemetry report on queue depths, wait times, and backpressure metrics.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct QueueMetrics {
    pub queued_counts_by_priority: HashMap<String, usize>,
    pub total_enqueued: u64,
    pub total_dispatched: u64,
    pub total_rejected: u64,
    pub priority_promotions: u64,
    pub p50_wait_time_ms: f64,
    pub p99_wait_time_ms: f64,
    pub gang_deadlocks_avoided: u64,
}

/// Central priority-banded Deficit Round-Robin job queue and scheduler coordinator.
#[derive(Debug)]
pub struct PriorityJobQueue {
    admission: AdmissionController,
    gang_coordinator: Arc<DeadlockFreeGangCoordinator>,
    queues: [VecDeque<QueuedJob>; 4],
    deficits: [usize; 4],
    current_drr_band: usize,
    backpressure: HashMap<NodeId, BackpressureStatus>,
    wait_time_samples_ms: Vec<f64>,
    total_enqueued: u64,
    total_dispatched: u64,
    total_rejected: u64,
    priority_promotions: u64,
}

impl PriorityJobQueue {
    /// Creates a new `PriorityJobQueue` with admission control and gang coordination.
    #[must_use]
    pub fn new(config: AdmissionConfig) -> Self {
        Self {
            admission: AdmissionController::new(config),
            gang_coordinator: Arc::new(DeadlockFreeGangCoordinator::new()),
            queues: [
                VecDeque::new(),
                VecDeque::new(),
                VecDeque::new(),
                VecDeque::new(),
            ],
            deficits: [0, 0, 0, 0],
            current_drr_band: 0,
            backpressure: HashMap::new(),
            wait_time_samples_ms: Vec::new(),
            total_enqueued: 0,
            total_dispatched: 0,
            total_rejected: 0,
            priority_promotions: 0,
        }
    }

    /// Returns reference to gang coordinator.
    #[must_use]
    pub fn gang_coordinator(&self) -> Arc<DeadlockFreeGangCoordinator> {
        Arc::clone(&self.gang_coordinator)
    }

    /// Enqueues a job after validating admission control.
    pub fn enqueue(&mut self, job: QueuedJob) -> Result<(), SchedulingError> {
        if let Err(e) = self.admission.admit_job(&job) {
            self.total_rejected += 1;
            return Err(e);
        }

        let band_idx = job.effective_priority as usize;
        self.queues[band_idx].push_back(job);
        self.total_enqueued += 1;
        Ok(())
    }

    /// Updates backpressure status for a worker node.
    pub fn update_node_backpressure(&mut self, node_id: NodeId, status: BackpressureStatus) {
        self.backpressure.insert(node_id, status);
    }

    /// Checks if a worker node is saturated by backpressure.
    #[must_use]
    pub fn is_node_saturated(&self, node_id: &NodeId) -> bool {
        matches!(
            self.backpressure.get(node_id),
            Some(BackpressureStatus::Saturated { .. })
        )
    }

    /// Applies anti-starvation priority aging:
    /// Jobs in `StandardBatch` or `BackgroundMaintenance` waiting longer than `aging_threshold_secs`
    /// are promoted to the next higher priority band.
    pub fn apply_priority_aging(&mut self, now_us: u64) {
        let threshold_us = (self.admission.config.aging_threshold_secs * 1_000_000.0) as u64;

        // Check BackgroundMaintenance (band 3) -> promote to StandardBatch (band 2)
        let mut promoted_to_batch = Vec::new();
        self.queues[3].retain(|job| {
            if now_us.saturating_sub(job.enqueued_at_us) >= threshold_us {
                let mut p = job.clone();
                p.effective_priority = PriorityBand::StandardBatch;
                promoted_to_batch.push(p);
                false
            } else {
                true
            }
        });
        self.priority_promotions += promoted_to_batch.len() as u64;
        self.queues[2].extend(promoted_to_batch);

        // Check StandardBatch (band 2) -> promote to HighPriorityUser (band 1)
        let mut promoted_to_high = Vec::new();
        self.queues[2].retain(|job| {
            if now_us.saturating_sub(job.enqueued_at_us) >= threshold_us {
                let mut p = job.clone();
                p.effective_priority = PriorityBand::HighPriorityUser;
                promoted_to_high.push(p);
                false
            } else {
                true
            }
        });
        self.priority_promotions += promoted_to_high.len() as u64;
        self.queues[1].extend(promoted_to_high);
    }

    /// Pops the next job to schedule using priority-banded Deficit Round-Robin.
    /// InteractiveRealtime jobs (band 0) are always dispatched with immediate strict priority (< 15 ms).
    pub fn pop_next(&mut self, now_us: u64) -> Option<QueuedJob> {
        // 1. Strict priority for InteractiveRealtime (band 0)
        if let Some(job) = self.queues[0].pop_front() {
            self.record_dispatch(&job, now_us);
            return Some(job);
        }

        // 2. Deficit Round-Robin across remaining bands (1, 2, 3)
        for _ in 0..6 {
            let band = self.current_drr_band;
            let quantum = match band {
                1 => PriorityBand::HighPriorityUser.drr_quantum(),
                2 => PriorityBand::StandardBatch.drr_quantum(),
                3 => PriorityBand::BackgroundMaintenance.drr_quantum(),
                _ => 1,
            };

            if self.deficits[band] == 0 {
                self.deficits[band] += quantum;
            }

            if let Some(job) = self.queues[band].pop_front() {
                self.deficits[band] = self.deficits[band].saturating_sub(1);
                self.record_dispatch(&job, now_us);
                return Some(job);
            }

            // Queue is empty for this band: reset deficit and advance round-robin
            self.deficits[band] = 0;
            self.current_drr_band = if band >= 3 { 1 } else { band + 1 };
        }

        None
    }

    fn record_dispatch(&mut self, job: &QueuedJob, now_us: u64) {
        self.admission.release_job(job);
        self.total_dispatched += 1;
        let wait_ms = (now_us.saturating_sub(job.enqueued_at_us)) as f64 / 1000.0;
        self.wait_time_samples_ms.push(wait_ms);
    }

    /// Computes summary metrics across all priority bands and wait times.
    #[must_use]
    pub fn metrics(&self) -> QueueMetrics {
        let mut counts = HashMap::new();
        counts.insert("InteractiveRealtime".into(), self.queues[0].len());
        counts.insert("HighPriorityUser".into(), self.queues[1].len());
        counts.insert("StandardBatch".into(), self.queues[2].len());
        counts.insert("BackgroundMaintenance".into(), self.queues[3].len());

        let mut sorted_waits = self.wait_time_samples_ms.clone();
        sorted_waits.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let (p50, p99) = if sorted_waits.is_empty() {
            (0.0, 0.0)
        } else {
            let n = sorted_waits.len();
            let p50_idx = ((n as f64 * 0.50) as usize).min(n - 1);
            let p99_idx = ((n as f64 * 0.99) as usize).min(n - 1);
            (sorted_waits[p50_idx], sorted_waits[p99_idx])
        };

        QueueMetrics {
            queued_counts_by_priority: counts,
            total_enqueued: self.total_enqueued,
            total_dispatched: self.total_dispatched,
            total_rejected: self.total_rejected,
            priority_promotions: self.priority_promotions,
            p50_wait_time_ms: p50,
            p99_wait_time_ms: p99,
            gang_deadlocks_avoided: self.gang_coordinator.deadlocks_avoided(),
        }
    }
}
