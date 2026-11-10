//! Split-brain partition reconciliation and Master election conflict resolver.
//! Merges diverged LAN sub-clusters upon WAN/interconnect restoration and resolves conflicting Master terms.
//! Strictly enforces `#![forbid(unsafe_code)]` and zero-placeholder invariants.

use std::time::Instant;

use serde::{Deserialize, Serialize};
use umcf_core::{NodeId, NodeRole};

use crate::error::RegistryError;
use crate::store::LocalRegistryStore;

/// Summary of a partition reconciliation event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReconciliationSummary {
    /// NodeId of the surviving Master.
    pub winner_node_id: NodeId,
    /// Election term of the surviving Master.
    pub winner_term: u64,
    /// Fitness score of the surviving Master.
    pub winner_fitness: u32,
    /// NodeId of the demoted former Master.
    pub demoted_node_id: NodeId,
    /// Election term of the demoted former Master.
    pub demoted_term: u64,
    /// Fitness score of the demoted former Master.
    pub demoted_fitness: u32,
    /// Duration of partition reconciliation in milliseconds.
    pub duration_ms: f64,
    /// Total node count in the reconciled cluster.
    pub reconciled_node_count: usize,
    /// Whether both partitions converged to byte-identical state.
    pub states_converged: bool,
}

/// Describes the leader state of a cluster island entering partition reconciliation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IslandLeaderInfo<'a> {
    pub master_id: &'a NodeId,
    pub term: u64,
    pub fitness: u32,
}

/// Reconciles two diverged cluster partitions after a network split heals.
///
/// Arbitration Rules:
/// 1. Higher election term strictly wins.
/// 2. If election terms are tied, higher `MasterFitnessScore` wins.
/// 3. If fitness scores are tied, deterministic lexicographical tie-breaking on `NodeId`.
/// 4. The losing Master is demoted to `NodeRole::Worker`.
/// 5. Causal CRDT deltas are bidirectionally exchanged, converging both islands to 100% identical state.
///
/// # Errors
/// Returns `RegistryError` if delta generation or persistence fails.
pub fn reconcile_partition(
    island_a: &mut LocalRegistryStore,
    leader_a: IslandLeaderInfo<'_>,
    island_b: &mut LocalRegistryStore,
    leader_b: IslandLeaderInfo<'_>,
) -> Result<ReconciliationSummary, RegistryError> {
    let start_time = Instant::now();

    // 1. Conflict Arbitration: determine winning and losing master
    let a_wins = if leader_a.term != leader_b.term {
        leader_a.term > leader_b.term
    } else if leader_a.fitness != leader_b.fitness {
        leader_a.fitness > leader_b.fitness
    } else {
        leader_a.master_id.as_str() > leader_b.master_id.as_str()
    };

    let (winner_id, winner_term, winner_fitness, loser_id, loser_term, loser_fitness) = if a_wins {
        (
            leader_a.master_id.clone(),
            leader_a.term,
            leader_a.fitness,
            leader_b.master_id.clone(),
            leader_b.term,
            leader_b.fitness,
        )
    } else {
        (
            leader_b.master_id.clone(),
            leader_b.term,
            leader_b.fitness,
            leader_a.master_id.clone(),
            leader_a.term,
            leader_a.fitness,
        )
    };

    // 2. Generate causal deltas from both islands
    let delta_a = island_a.generate_delta(None)?;
    let delta_b = island_b.generate_delta(None)?;

    // 3. Bidirectional delta merge
    island_a.apply_delta(&delta_b)?;
    island_b.apply_delta(&delta_a)?;

    // 4. Apply role adjustments canonically on Island A
    if let Some(mut loser_rec) = island_a.get_node(&loser_id) {
        loser_rec.role = NodeRole::Worker;
        island_a.register_node(loser_rec)?;
    }

    if let Some(mut winner_rec) = island_a.get_node(&winner_id) {
        winner_rec.role = NodeRole::Master;
        island_a.register_node(winner_rec)?;
    }

    // 5. Replicate canonical role adjustments to Island B via delta
    let reconciliation_delta = island_a.generate_delta(None)?;
    island_b.apply_delta(&reconciliation_delta)?;

    // 6. Verify convergence parity
    let state_a = island_a.cluster_state();
    let state_b = island_b.cluster_state();
    let states_converged = state_a == state_b;

    let elapsed = start_time.elapsed();
    let duration_ms = elapsed.as_secs_f64() * 1000.0;

    let node_count = island_a.list_active_nodes().len();

    Ok(ReconciliationSummary {
        winner_node_id: winner_id,
        winner_term,
        winner_fitness,
        demoted_node_id: loser_id,
        demoted_term: loser_term,
        demoted_fitness: loser_fitness,
        duration_ms,
        reconciled_node_count: node_count,
        states_converged,
    })
}
