//! `umcf-scheduler::fault_tolerance`: Failover coordination, checkpoint-aware rescheduling,
//! and speculative backup assignment for straggler workers.
//! Strictly enforces `#![forbid(unsafe_code)]` and zero placeholders.

#![forbid(unsafe_code)]

use std::collections::{HashMap, HashSet};
use std::sync::RwLock;

use umcf_core::NodeId;

use crate::cost::NodeCandidateProfile;
use crate::dag::TaskNode;
use crate::error::{SchedulingError, UnschedulableReason};
use crate::solver::{ConstraintSolver, PlacementDecision};

/// Status of an active task execution attempt.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum TaskAttemptStatus {
    Running,
    SpeculativeBackupRunning,
    Succeeded,
    Failed { reason: String },
    Preempted,
    CancelledDueToSpeculativeWinner,
}

/// Tracking metadata for a task execution attempt.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TaskAttempt {
    pub attempt_id: u32,
    pub worker_node_id: NodeId,
    pub is_speculative: bool,
    pub started_at_step: u64,
    pub status: TaskAttemptStatus,
}

/// State tracking for a task across potential failovers and speculative attempts.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TaskExecutionState {
    pub task_id: String,
    pub attempts: Vec<TaskAttempt>,
    pub max_attempts: u32,
    pub excluded_nodes: HashSet<NodeId>,
    pub latest_checkpoint_step: u64,
}

/// Coordinator for task failover and speculative backup scheduling.
#[derive(Debug)]
pub struct TaskFailoverCoordinator {
    task_states: RwLock<HashMap<String, TaskExecutionState>>,
    solver: ConstraintSolver,
    max_retry_attempts: u32,
    local_node_id: NodeId,
}

impl TaskFailoverCoordinator {
    /// Creates a new coordinator with specified local node ID and max retry attempts (default: 3).
    #[must_use]
    pub fn new(local_node_id: NodeId, max_retry_attempts: u32) -> Self {
        Self {
            task_states: RwLock::new(HashMap::new()),
            solver: ConstraintSolver::new(),
            max_retry_attempts,
            local_node_id,
        }
    }

    /// Registers a newly scheduled task with its initial worker assignment.
    pub fn register_initial_task(
        &self,
        task_id: impl Into<String>,
        worker_node_id: NodeId,
    ) {
        let task_id = task_id.into();
        let initial_attempt = TaskAttempt {
            attempt_id: 1,
            worker_node_id,
            is_speculative: false,
            started_at_step: 0,
            status: TaskAttemptStatus::Running,
        };

        let state = TaskExecutionState {
            task_id: task_id.clone(),
            attempts: vec![initial_attempt],
            max_attempts: self.max_retry_attempts,
            excluded_nodes: HashSet::new(),
            latest_checkpoint_step: 0,
        };

        if let Ok(mut map) = self.task_states.write() {
            map.insert(task_id, state);
        }
    }

    /// Handles worker failure or ungraceful disconnect by rescheduling on the next-best healthy peer.
    /// Excludes the failed worker, increments attempt counter, and resumes from `checkpoint_step`.
    pub fn handle_worker_failure(
        &self,
        task: &TaskNode,
        failed_worker_id: &NodeId,
        checkpoint_step: u64,
        candidate_pool: &[NodeCandidateProfile],
    ) -> Result<PlacementDecision, SchedulingError> {
        let mut map = match self.task_states.write() {
            Ok(guard) => guard,
            Err(_) => {
                return Err(SchedulingError::UnschedulableTask {
                    task_id: task.task_id.clone(),
                    reason: UnschedulableReason::NoFeasibleNodes,
                })
            }
        };

        let state = map
            .entry(task.task_id.clone())
            .or_insert_with(|| TaskExecutionState {
                task_id: task.task_id.clone(),
                attempts: Vec::new(),
                max_attempts: self.max_retry_attempts,
                excluded_nodes: HashSet::new(),
                latest_checkpoint_step: checkpoint_step,
            });

        // Check max attempts
        if state.attempts.len() as u32 >= state.max_attempts {
            return Err(SchedulingError::UnschedulableTask {
                task_id: task.task_id.clone(),
                reason: UnschedulableReason::NoFeasibleNodes,
            });
        }

        // Exclude failed worker from future attempts
        state.excluded_nodes.insert(failed_worker_id.clone());
        state.latest_checkpoint_step = checkpoint_step;

        // Filter candidate pool excluding dead nodes
        let available_candidates: Vec<NodeCandidateProfile> = candidate_pool
            .iter()
            .filter(|c| !state.excluded_nodes.contains(&c.node_id))
            .cloned()
            .collect();

        let feasible = self
            .solver
            .filter_feasible_nodes(task, &available_candidates, &self.local_node_id)
            .map_err(|reason| SchedulingError::UnschedulableTask {
                task_id: task.task_id.clone(),
                reason,
            })?;

        let decision = self
            .solver
            .schedule_task(task, &feasible, &self.local_node_id, 0)
            .map_err(|reason| SchedulingError::UnschedulableTask {
                task_id: task.task_id.clone(),
                reason,
            })?;

        let attempt_id = (state.attempts.len() as u32) + 1;
        state.attempts.push(TaskAttempt {
            attempt_id,
            worker_node_id: decision.assigned_node_id.clone(),
            is_speculative: false,
            started_at_step: checkpoint_step,
            status: TaskAttemptStatus::Running,
        });

        Ok(decision)
    }

    /// Launches a speculative backup task for a detected straggler on an available peer.
    /// Does not preempt original worker yet, allowing either to win the commit race.
    pub fn handle_straggler_speculative_backup(
        &self,
        task: &TaskNode,
        slow_worker_id: &NodeId,
        checkpoint_step: u64,
        candidate_pool: &[NodeCandidateProfile],
    ) -> Result<Option<PlacementDecision>, SchedulingError> {
        let mut map = match self.task_states.write() {
            Ok(guard) => guard,
            Err(_) => return Ok(None),
        };

        let state = match map.get_mut(&task.task_id) {
            Some(s) => s,
            None => return Ok(None),
        };

        // Don't launch multiple speculative backups if one is already active
        let already_has_speculative = state
            .attempts
            .iter()
            .any(|a| a.is_speculative && a.status == TaskAttemptStatus::SpeculativeBackupRunning);
        if already_has_speculative {
            return Ok(None);
        }

        // Filter out slow worker
        let available_candidates: Vec<NodeCandidateProfile> = candidate_pool
            .iter()
            .filter(|c| c.node_id != *slow_worker_id && !state.excluded_nodes.contains(&c.node_id))
            .cloned()
            .collect();

        if available_candidates.is_empty() {
            return Ok(None);
        }

        let feasible = match self
            .solver
            .filter_feasible_nodes(task, &available_candidates, &self.local_node_id)
        {
            Ok(f) => f,
            Err(_) => return Ok(None),
        };

        let decision = match self
            .solver
            .schedule_task(task, &feasible, &self.local_node_id, 0)
        {
            Ok(d) => d,
            Err(_) => return Ok(None),
        };

        let attempt_id = (state.attempts.len() as u32) + 1;
        state.attempts.push(TaskAttempt {
            attempt_id,
            worker_node_id: decision.assigned_node_id.clone(),
            is_speculative: true,
            started_at_step: checkpoint_step,
            status: TaskAttemptStatus::SpeculativeBackupRunning,
        });

        Ok(Some(decision))
    }

    /// Records completion of a task attempt, marking the task succeeded and cancelling losers.
    pub fn record_task_completion(
        &self,
        task_id: &str,
        winning_worker_id: &NodeId,
    ) -> Option<Vec<NodeId>> {
        let mut map = self.task_states.write().ok()?;
        let state = map.get_mut(task_id)?;

        let mut losers_to_cancel = Vec::new();
        for attempt in &mut state.attempts {
            if attempt.worker_node_id == *winning_worker_id {
                attempt.status = TaskAttemptStatus::Succeeded;
            } else if attempt.status == TaskAttemptStatus::Running
                || attempt.status == TaskAttemptStatus::SpeculativeBackupRunning
            {
                attempt.status = TaskAttemptStatus::CancelledDueToSpeculativeWinner;
                losers_to_cancel.push(attempt.worker_node_id.clone());
            }
        }

        Some(losers_to_cancel)
    }

    /// Returns the task execution state snapshot.
    #[must_use]
    pub fn get_task_state(&self, task_id: &str) -> Option<TaskExecutionState> {
        self.task_states
            .read()
            .ok()
            .and_then(|guard| guard.get(task_id).cloned())
    }
}
