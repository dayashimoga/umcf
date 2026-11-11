//! Continuous Invariant Monitor verifying safety and liveness during chaos campaigns.

use std::collections::{HashMap, HashSet};
use crate::error::ChaosError;

/// Cluster Invariant Monitor.
#[derive(Debug, Clone, Default)]
pub struct ClusterInvariantMonitor {
    pub settled_receipt_ids: HashSet<String>,
}

impl ClusterInvariantMonitor {
    pub fn new() -> Self {
        Self {
            settled_receipt_ids: HashSet::new(),
        }
    }

    /// Invariant 1: No Silent Data or Compute Corruption.
    /// Assert that every completed job output matches the expected cryptographic BLAKE3 digest.
    pub fn check_invariant_1_data_integrity(
        &self,
        job_id: &str,
        expected_blake3_hex: &str,
        actual_blake3_hex: &str,
    ) -> Result<(), ChaosError> {
        if expected_blake3_hex != actual_blake3_hex {
            return Err(ChaosError::DataCorruptionDetected {
                job_id: job_id.to_string(),
                expected: expected_blake3_hex.to_string(),
                actual: actual_blake3_hex.to_string(),
            });
        }
        Ok(())
    }

    /// Invariant 2: Bounded Split-Brain Convergence.
    /// Within <= 5.0 seconds of partition healing, all nodes must converge on an identical view.
    pub fn check_invariant_2_split_brain_convergence(
        &self,
        node_views: &HashMap<String, Vec<String>>,
        heal_duration_ms: u64,
        max_allowed_ms: u64,
    ) -> Result<(), ChaosError> {
        if heal_duration_ms > max_allowed_ms {
            return Err(ChaosError::SplitBrainConvergenceTimeout {
                elapsed_ms: heal_duration_ms,
                max_allowed_ms,
            });
        }

        if node_views.is_empty() {
            return Ok(());
        }

        // Compare all views to the first view
        let first_view: HashSet<_> = node_views.values().next().map(|v| v.iter().cloned().collect()).unwrap_or_default();
        for (node, view) in node_views {
            let current_view: HashSet<_> = view.iter().cloned().collect();
            if current_view != first_view {
                return Err(ChaosError::InvariantViolation {
                    invariant_id: 2,
                    reason: format!(
                        "Node '{node}' view ({:?}) diverges from cluster baseline ({:?}) after {heal_duration_ms} ms",
                        view, first_view
                    ),
                });
            }
        }
        Ok(())
    }

    /// Invariant 3: No Orphaned / Zombie Workloads.
    /// Preempted or partitioned tasks on isolated nodes must self-abort within their lease heartbeat timeout (<= 3.0 s).
    pub fn check_invariant_3_no_zombie_workloads(
        &self,
        active_tasks: &[(&str, &str, u64)], // (task_id, node_id, last_heartbeat_age_ms)
        lease_timeout_ms: u64,
    ) -> Result<(), ChaosError> {
        for &(task_id, node_id, heartbeat_age_ms) in active_tasks {
            if heartbeat_age_ms > lease_timeout_ms {
                return Err(ChaosError::ZombieTaskDetected {
                    task_id: task_id.to_string(),
                    node_id: node_id.to_string(),
                    lease_age_ms: heartbeat_age_ms,
                });
            }
        }
        Ok(())
    }

    /// Invariant 4: Storage Durability.
    /// Objects stored with RS(k, m) remain readable as long as at least k shards survive.
    pub fn check_invariant_4_storage_durability(
        &self,
        cid: &str,
        surviving_shards: usize,
        min_required_k: usize,
    ) -> Result<(), ChaosError> {
        if surviving_shards < min_required_k {
            return Err(ChaosError::StorageDurabilityLost {
                cid: cid.to_string(),
                surviving_shards,
                min_required: min_required_k,
            });
        }
        Ok(())
    }

    /// Invariant 5: Strict Ledger Conservation & Idempotent Settlement.
    /// Double-payouts are rejected, and debit/credit conservation must hold.
    pub fn check_invariant_5_ledger_conservation(
        &mut self,
        receipt_id: &str,
        total_debits: i128,
        total_credits: i128,
    ) -> Result<(), ChaosError> {
        // 1. Balance conservation
        if total_debits != total_credits {
            return Err(ChaosError::InvariantViolation {
                invariant_id: 5,
                reason: format!("Journal unbalance: total debits ({total_debits}) != total credits ({total_credits})"),
            });
        }

        // 2. Receipt idempotency
        if self.settled_receipt_ids.contains(receipt_id) {
            return Err(ChaosError::LedgerDoublePayout {
                receipt_id: receipt_id.to_string(),
            });
        }

        self.settled_receipt_ids.insert(receipt_id.to_string());
        Ok(())
    }
}
