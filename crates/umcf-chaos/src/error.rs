//! Structured error types for Chaos Engineering & Fault-Injection.

use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ChaosError {
    #[error("Invariant {invariant_id} violation: {reason}")]
    InvariantViolation { invariant_id: u8, reason: String },

    #[error("Split-brain convergence timed out: elapsed {elapsed_ms} ms exceeds limit of {max_allowed_ms} ms")]
    SplitBrainConvergenceTimeout {
        elapsed_ms: u64,
        max_allowed_ms: u64,
    },

    #[error("Zombie/orphaned task detected: task '{task_id}' on node '{node_id}' active after lease expiry ({lease_age_ms} ms)")]
    ZombieTaskDetected {
        task_id: String,
        node_id: String,
        lease_age_ms: u64,
    },

    #[error("Data corruption detected on job '{job_id}': expected digest {expected}, actual digest {actual}")]
    DataCorruptionDetected {
        job_id: String,
        expected: String,
        actual: String,
    },

    #[error("Ledger double-payout detected: receipt '{receipt_id}' settled multiple times")]
    LedgerDoublePayout { receipt_id: String },

    #[error("Storage durability lost for CID '{cid}': only {surviving_shards} shards survive (minimum {min_required} required)")]
    StorageDurabilityLost {
        cid: String,
        surviving_shards: usize,
        min_required: usize,
    },

    #[error("Chaos nemesis '{nemesis_name}' execution failed: {reason}")]
    NemesisExecutionFailed {
        nemesis_name: String,
        reason: String,
    },

    #[error("Task migration failed for node '{node_id}': {reason}")]
    TaskMigrationFailed { node_id: String, reason: String },
}
