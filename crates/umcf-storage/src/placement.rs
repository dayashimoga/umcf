//! Failure-Domain-Aware Shard Placement, Durability Auditing & Self-Healing Repair Worker.
//!
//! Subphase S7.P1.SP2:
//! - Multi-attribute node scoring (failure domain diversity, AC power vs battery, uptime score, quota headroom).
//! - Strict parity tolerance cap: no single node holds more than floor(M/2) + 1 shards when cluster size >= K + M.
//! - Self-healing repair worker restoring K+M durability when nodes churn or depart permanently.

#![forbid(unsafe_code)]

use crate::error::StorageError;
use crate::erasure::{EncryptedShard, RedundancyProfile, ReedSolomonErasureEngine};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use umcf_core::types::NodeId;

/// Descriptor of a candidate storage node in the compute mesh.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StorageNodeDescriptor {
    /// Unique cryptographic node identifier.
    pub node_id: NodeId,
    /// Physical or network failure domain (e.g. "subnet:192.168.1.0/24", "rack:alpha", "host:desktop-1").
    pub failure_domain: String,
    /// Whether node is currently connected to AC power or dock (true), or running on battery (false).
    pub is_ac_powered: bool,
    /// Current battery charge percentage (0.0 to 100.0).
    pub battery_pct: f32,
    /// Historical availability and uptime reliability score (0.0 to 1.0).
    pub uptime_score: f64,
    /// Available storage quota headroom in bytes.
    pub available_quota_bytes: u64,
    /// Total configured storage quota in bytes.
    pub total_quota_bytes: u64,
}

impl StorageNodeDescriptor {
    /// Computes candidate node fitness score (higher is more reliable and preferred).
    #[must_use]
    pub fn compute_fitness_score(&self) -> f64 {
        let mut score = 0.0;

        // 1. Power stability: AC power provides +50 base score; battery scales with charge
        if self.is_ac_powered {
            score += 50.0;
        } else {
            score += (self.battery_pct.clamp(0.0, 100.0) as f64) * 0.25; // 0..25 pts
        }

        // 2. Historical uptime reliability (0..30 pts)
        score += self.uptime_score.clamp(0.0, 1.0) * 30.0;

        // 3. Storage quota headroom ratio (0..20 pts)
        if self.total_quota_bytes > 0 {
            let ratio = (self.available_quota_bytes as f64) / (self.total_quota_bytes as f64);
            score += ratio.clamp(0.0, 1.0) * 20.0;
        }

        score
    }
}

/// Failure-domain-aware shard placement engine.
#[derive(Debug, Clone, Default)]
pub struct FailureDomainPlacementEngine;

impl FailureDomainPlacementEngine {
    /// Selects optimal target nodes for placing total_shards across independent failure domains.
    ///
    /// Guarantees:
    /// 1. When candidates >= total_shards, each node holds at most 1 shard.
    /// 2. Parity constraint: No single node holds more than floor(M / 2) + 1 shards when candidates >= K + M.
    /// 3. Prefer distinct failure domains and AC-powered nodes with high reliability.
    pub fn select_target_nodes(
        total_shards: usize,
        parity_shards_m: usize,
        candidates: &[StorageNodeDescriptor],
    ) -> Result<Vec<NodeId>, StorageError> {
        if candidates.is_empty() {
            return Err(StorageError::PlacementError {
                reason: "no candidate storage nodes available in cluster".to_string(),
            });
        }
        if total_shards == 0 {
            return Err(StorageError::PlacementError {
                reason: "total_shards must be > 0".to_string(),
            });
        }

        // Calculate max allowed shards per node
        let max_shards_per_node = if candidates.len() >= total_shards {
            1
        } else {
            ((parity_shards_m / 2) + 1).max(1)
        };

        if candidates.len() * max_shards_per_node < total_shards {
            return Err(StorageError::PlacementError {
                reason: format!(
                    "insufficient cluster capacity: need {total_shards} shard placements, but {} nodes with max {max_shards_per_node} per node provide only {}",
                    candidates.len(),
                    candidates.len() * max_shards_per_node
                ),
            });
        }

        // Group candidates by failure domain
        let mut domain_map: HashMap<String, Vec<&StorageNodeDescriptor>> = HashMap::new();
        for node in candidates {
            domain_map.entry(node.failure_domain.clone()).or_default().push(node);
        }

        // Sort candidates within each failure domain by descending fitness score
        for nodes in domain_map.values_mut() {
            nodes.sort_by(|a, b| {
                b.compute_fitness_score()
                    .partial_cmp(&a.compute_fitness_score())
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        }

        // Collect unique domains and sort by count of available nodes descending
        let mut domains: Vec<String> = domain_map.keys().cloned().collect();
        domains.sort_by(|a, b| {
            let len_a = domain_map.get(a).map_or(0, |v| v.len());
            let len_b = domain_map.get(b).map_or(0, |v| v.len());
            len_b.cmp(&len_a)
        });

        let mut assignments: Vec<NodeId> = Vec::with_capacity(total_shards);
        let mut node_allocation_counts: HashMap<NodeId, usize> = HashMap::new();
        let mut domain_idx = 0;

        while assignments.len() < total_shards {
            let mut placed_in_round = false;
            let start_domain_idx = domain_idx;

            loop {
                let domain = &domains[domain_idx];
                domain_idx = (domain_idx + 1) % domains.len();

                if let Some(nodes) = domain_map.get(domain) {
                    // Find highest scoring node in this domain that has not exceeded quota
                    for candidate in nodes {
                        let current_count = node_allocation_counts.get(&candidate.node_id).copied().unwrap_or(0);
                        if current_count < max_shards_per_node {
                            assignments.push(candidate.node_id.clone());
                            *node_allocation_counts.entry(candidate.node_id.clone()).or_default() += 1;
                            placed_in_round = true;
                            break;
                        }
                    }
                }

                if placed_in_round || domain_idx == start_domain_idx {
                    break;
                }
            }

            if !placed_in_round {
                // If round-robin domain rotation cannot find a free slot, greedy search any node
                let mut best_candidate: Option<(&StorageNodeDescriptor, f64)> = None;
                for candidate in candidates {
                    let current_count = node_allocation_counts.get(&candidate.node_id).copied().unwrap_or(0);
                    if current_count < max_shards_per_node {
                        let score = candidate.compute_fitness_score();
                        if best_candidate.as_ref().map_or(true, |(_, best_s)| score > *best_s) {
                            best_candidate = Some((candidate, score));
                        }
                    }
                }

                if let Some((best, _)) = best_candidate {
                    assignments.push(best.node_id.clone());
                    *node_allocation_counts.entry(best.node_id.clone()).or_default() += 1;
                } else {
                    return Err(StorageError::PlacementError {
                        reason: "exhausted all eligible candidate node placements".to_string(),
                    });
                }
            }
        }

        Ok(assignments)
    }
}

/// Durability status of an object's shards in the compute mesh.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DurabilityState {
    /// All K + M shards are online and healthy.
    Healthy,
    /// Between K and K + M - 1 shards are reachable. Data is safe but redundancy is degraded.
    Degraded,
    /// Fewer than K shards are reachable. Data cannot be recovered.
    Unrecoverable,
}

/// Durability audit health report for an object.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShardHealthReport {
    /// Content identifier of the object.
    pub object_cid: String,
    /// Durability state.
    pub state: DurabilityState,
    /// Number of active reachable shards.
    pub active_shards: usize,
    /// Required data shards K.
    pub required_shards_k: usize,
    /// Total shards (K + M).
    pub total_shards: usize,
    /// Number of additional node outages tolerated before data loss.
    pub tolerates_outages: usize,
    /// Human-readable diagnostic status badge.
    pub status_badge: String,
}

/// A recorded self-healing repair event.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RepairEvent {
    /// Timestamp in milliseconds since Unix epoch.
    pub timestamp_ms: u64,
    /// Object CID repaired.
    pub object_cid: String,
    /// Shard indices that were missing and reconstructed.
    pub repaired_shard_indices: Vec<u8>,
    /// Target nodes that received the reconstructed shards.
    pub new_target_nodes: Vec<NodeId>,
    /// Status transition (e.g. "Degraded (5/6 Shards) -> Healing -> Healthy (6/6)").
    pub transition_summary: String,
}

/// Stored object shard placement record tracked by the cluster.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ObjectPlacementRecord {
    /// Object CID.
    pub object_cid: String,
    /// Plaintext BLAKE3 hash.
    pub plaintext_blake3: [u8; 32],
    /// Redundancy profile.
    pub redundancy_profile: RedundancyProfile,
    /// Encrypted shards.
    pub shards: Vec<EncryptedShard>,
    /// Map of shard_index -> assigned NodeId.
    pub node_placements: HashMap<u8, NodeId>,
}

/// Self-healing shard repair coordinator.
#[derive(Debug, Clone, Default)]
pub struct ShardHealthRepairWorker {
    objects: HashMap<String, ObjectPlacementRecord>,
}

impl ShardHealthRepairWorker {
    /// Creates a new self-healing repair worker.
    #[must_use]
    pub fn new() -> Self {
        Self {
            objects: HashMap::new(),
        }
    }

    /// Registers an object's placement and shards for durability tracking.
    pub fn register_object(&mut self, record: ObjectPlacementRecord) {
        self.objects.insert(record.object_cid.clone(), record);
    }

    /// Audits the live durability health of an object given the currently reachable mesh nodes.
    pub fn audit_object_health(
        &self,
        object_cid: &str,
        active_nodes: &HashSet<NodeId>,
    ) -> Result<ShardHealthReport, StorageError> {
        let record = self.objects.get(object_cid).ok_or_else(|| StorageError::ManifestNotFound {
            root_cid: object_cid.to_string(),
        })?;

        let k = record.redundancy_profile.data_shards_k();
        let total = record.redundancy_profile.total_shards();

        let mut active_count = 0usize;
        for (shard_idx, assigned_node) in &record.node_placements {
            if active_nodes.contains(assigned_node) && (*shard_idx as usize) < record.shards.len() {
                active_count += 1;
            }
        }

        let state = if active_count >= total {
            DurabilityState::Healthy
        } else if active_count >= k {
            DurabilityState::Degraded
        } else {
            DurabilityState::Unrecoverable
        };

        let tolerates = active_count.saturating_sub(k);
        let status_badge = match state {
            DurabilityState::Healthy => {
                format!("{active_count}/{total} Shards Healthy — Tolerates {tolerates} Node Outages")
            }
            DurabilityState::Degraded => {
                format!("Degraded ({active_count}/{total} Shards) — Tolerates {tolerates} Outages (Repair Required)")
            }
            DurabilityState::Unrecoverable => {
                format!("Unrecoverable ({active_count}/{total} Shards) — Data Loss Threshold Exceeded")
            }
        };

        Ok(ShardHealthReport {
            object_cid: object_cid.to_string(),
            state,
            active_shards: active_count,
            required_shards_k: k,
            total_shards: total,
            tolerates_outages: tolerates,
            status_badge,
        })
    }

    /// Triggers self-healing repair for any degraded objects whose available shards have dropped.
    ///
    /// Reconstructs missing shards and places them onto eligible spare nodes.
    pub fn audit_and_repair_all(
        &mut self,
        active_nodes: &HashSet<NodeId>,
        candidates: &[StorageNodeDescriptor],
    ) -> Result<Vec<RepairEvent>, StorageError> {
        let mut events = Vec::new();
        let cids: Vec<String> = self.objects.keys().cloned().collect();

        for cid in cids {
            let health = self.audit_object_health(&cid, active_nodes)?;
            if health.state == DurabilityState::Degraded {
                let event = self.repair_single_object(&cid, active_nodes, candidates)?;
                events.push(event);
            }
        }

        Ok(events)
    }

    /// Performs self-healing repair on a specific degraded object.
    pub fn repair_single_object(
        &mut self,
        object_cid: &str,
        active_nodes: &HashSet<NodeId>,
        candidates: &[StorageNodeDescriptor],
    ) -> Result<RepairEvent, StorageError> {
        let record = self.objects.get_mut(object_cid).ok_or_else(|| StorageError::ManifestNotFound {
            root_cid: object_cid.to_string(),
        })?;

        let k = record.redundancy_profile.data_shards_k();
        let total = record.redundancy_profile.total_shards();
        let engine = ReedSolomonErasureEngine::new(record.redundancy_profile)?;

        // Identify missing shards and available shards
        let mut available_shards_slot: Vec<Option<EncryptedShard>> = vec![None; total];
        let mut missing_indices = Vec::new();
        let mut currently_used_nodes = HashSet::new();

        for (i, slot) in available_shards_slot.iter_mut().enumerate().take(total) {
            let shard_idx = i as u8;
            if let Some(assigned_node) = record.node_placements.get(&shard_idx) {
                if active_nodes.contains(assigned_node) {
                    if let Some(shard) = record.shards.get(i) {
                        *slot = Some(shard.clone());
                        currently_used_nodes.insert(assigned_node.clone());
                    }
                } else {
                    missing_indices.push(shard_idx);
                }
            } else {
                missing_indices.push(shard_idx);
            }
        }

        let before_active = total - missing_indices.len();
        if before_active < k {
            return Err(StorageError::InsufficientShards {
                required: k,
                available: before_active,
            });
        }

        // Filter candidate nodes to find spare nodes not currently holding any shard of this object
        let spare_candidates: Vec<StorageNodeDescriptor> = candidates
            .iter()
            .filter(|c| active_nodes.contains(&c.node_id) && !currently_used_nodes.contains(&c.node_id))
            .cloned()
            .collect();

        if spare_candidates.is_empty() {
            return Err(StorageError::RepairError {
                reason: "no eligible spare candidate nodes available for shard placement".to_string(),
            });
        }

        // Reconstruct each missing shard and allocate to a new healthy spare node
        let mut new_nodes = Vec::new();

        for (spare_idx, &missing_idx) in missing_indices.iter().enumerate() {
            let reconstructed = engine.reconstruct_missing_shard(&available_shards_slot, missing_idx)?;
            // Verify integrity of reconstructed shard
            reconstructed.verify_integrity()?;

            // Select spare node
            let target_node = spare_candidates[spare_idx % spare_candidates.len()].node_id.clone();

            // Update in-memory record
            if (missing_idx as usize) < record.shards.len() {
                record.shards[missing_idx as usize] = reconstructed.clone();
            } else {
                record.shards.push(reconstructed.clone());
            }
            record.node_placements.insert(missing_idx, target_node.clone());
            available_shards_slot[missing_idx as usize] = Some(reconstructed);

            new_nodes.push(target_node);
        }

        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        let transition = format!(
            "Degraded ({before_active}/{total} Shards) -> Healing -> Healthy ({total}/{total})"
        );

        Ok(RepairEvent {
            timestamp_ms: now_ms,
            object_cid: object_cid.to_string(),
            repaired_shard_indices: missing_indices,
            new_target_nodes: new_nodes,
            transition_summary: transition,
        })
    }

    /// Retrieves an object's placement record.
    #[must_use]
    pub fn get_object(&self, object_cid: &str) -> Option<&ObjectPlacementRecord> {
        self.objects.get(object_cid)
    }
}
