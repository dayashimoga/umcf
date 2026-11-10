//! `umcf-scheduler::error`: Structured errors and unschedulable diagnostic reasons.
//! Enforces `#![forbid(unsafe_code)]` and zero placeholders.

#![forbid(unsafe_code)]

use thiserror::Error;
use umcf_core::NodeId;

/// Error variants encountered during DAG compilation, constraint solving, or scheduling.
#[derive(Debug, Error, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SchedulingError {
    #[error("Cycle detected in JobDag: task '{0}' causes circular dependency")]
    CycleInDag(String),

    #[error("Referenced task '{0}' not found in JobDag")]
    TaskNotFound(String),

    #[error("Task '{task_id}' cannot be scheduled: {reason}")]
    UnschedulableTask {
        task_id: String,
        reason: UnschedulableReason,
    },

    #[error("JobDag validation failed: {0}")]
    InvalidDag(String),

    #[error("JobDag has no tasks")]
    EmptyDag,

    #[error("Cluster has zero registered candidate nodes")]
    NoNodesInCluster,
}

/// Detailed diagnostic explanation for why a task could not be scheduled on any node.
#[derive(Error, Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum UnschedulableReason {
    #[error("Violates PrivateAiLocalOnly policy: cannot place private task on remote node '{0}'")]
    PrivacyViolation(NodeId),

    #[error("Node '{0}' is suspended due to thermal or battery limits (PausedWaitCoolOrCharge / EmergencyEvict)")]
    ThermalOrBatteryHalt(NodeId),

    #[error("Required compute backend not supported by any eligible node")]
    MissingBackend,

    #[error("Insufficient consented RAM: required {required_bytes} bytes, available {available_bytes} bytes on node '{node_id}'")]
    InsufficientRam {
        node_id: NodeId,
        required_bytes: u64,
        available_bytes: u64,
    },

    #[error("Insufficient consented VRAM: required {required_bytes} bytes, available {available_bytes} bytes on node '{node_id}'")]
    InsufficientVram {
        node_id: NodeId,
        required_bytes: u64,
        available_bytes: u64,
    },

    #[error("Required instruction set (SIMD) '{0}' missing")]
    MissingSimd(String),

    #[error("Node role '{0:?}' not permitted for task")]
    DisallowedRole(String),

    #[error("No feasible candidate nodes satisfy all hard constraints")]
    NoFeasibleNodes,
}
