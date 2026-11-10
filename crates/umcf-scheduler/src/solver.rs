//! `umcf-scheduler::solver`: Multi-dimensional hardware constraint solver and optimal placement planner.
//! Enforces `#![forbid(unsafe_code)]` and zero placeholders.

#![forbid(unsafe_code)]

use std::collections::HashMap;
use std::time::Instant;
use umcf_core::{ComputeBackend, NodeId};
use umcf_guard::GuardRegime;

use crate::cost::{CostBreakdown, NodeCandidateProfile, RooflineCostModel};
use crate::dag::{JobDag, TaskNode};
use crate::error::{SchedulingError, UnschedulableReason};

/// Concrete placement assignment decision for a single task.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PlacementDecision {
    pub task_id: String,
    pub assigned_node_id: NodeId,
    pub selected_backend: ComputeBackend,
    pub cost_breakdown: CostBreakdown,
    pub is_local_execution: bool,
    pub reason: String,
}

/// Global placement plan for an entire `JobDag`.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PlacementPlan {
    pub job_id: String,
    pub assignments: HashMap<String, PlacementDecision>,
    pub topological_task_order: Vec<String>,
    pub total_estimated_makespan_ms: f64,
    pub solver_duration_micros: u64,
}

/// Capability-negotiated constraint solver and scheduler.
#[derive(Debug, Default)]
pub struct ConstraintSolver;

impl ConstraintSolver {
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Prunes candidate nodes that violate any hard constraint:
    /// - Privacy boundary (`PrivateAiLocalOnly`)
    /// - Thermal/battery regime (`PausedWaitCoolOrCharge`, `EmergencyEvict`)
    /// - Required compute backend
    /// - Insufficient consented RAM / VRAM
    /// - Missing instruction set / SIMD flags
    /// - Disallowed node role or explicit node ID blocklist
    pub fn filter_feasible_nodes<'a>(
        &self,
        task: &TaskNode,
        cluster: &'a [NodeCandidateProfile],
        local_node_id: &NodeId,
    ) -> Result<Vec<&'a NodeCandidateProfile>, UnschedulableReason> {
        let is_private = task.constraints.hard.privacy_local_only || task.spec.is_private_ai_local_only;
        let mut feasible = Vec::with_capacity(cluster.len());
        let mut last_rejection_reason = None;

        for node in cluster {
            // 1. Privacy boundary: private tasks MUST remain strictly on local node
            if is_private && node.node_id != *local_node_id {
                last_rejection_reason = Some(UnschedulableReason::PrivacyViolation(node.node_id.clone()));
                continue;
            }

            // 2. Thermal / Battery halt: node cannot accept workloads if paused or emergency evict
            if node.thermal_regime == GuardRegime::PausedWaitCoolOrCharge
                || node.thermal_regime == GuardRegime::EmergencyEvict
            {
                last_rejection_reason = Some(UnschedulableReason::ThermalOrBatteryHalt(node.node_id.clone()));
                continue;
            }

            // 3. Allowed node roles
            if !task.constraints.hard.allowed_node_roles.contains(&node.role) {
                last_rejection_reason = Some(UnschedulableReason::DisallowedRole(format!("{:?}", node.role)));
                continue;
            }

            // 4. Disallowed node blocklist
            if task.constraints.hard.disallowed_node_ids.contains(&node.node_id) {
                continue;
            }

            // 5. Consented RAM requirement
            let min_ram = task.constraints.hard.min_ram_bytes.max(task.spec.memory_bytes);
            if node.available_ram_bytes < min_ram {
                last_rejection_reason = Some(UnschedulableReason::InsufficientRam {
                    node_id: node.node_id.clone(),
                    required_bytes: min_ram,
                    available_bytes: node.available_ram_bytes,
                });
                continue;
            }

            // 6. Consented VRAM requirement (if task requests GPU/VRAM)
            if task.constraints.hard.min_vram_bytes > 0
                && node.available_vram_bytes < task.constraints.hard.min_vram_bytes
            {
                last_rejection_reason = Some(UnschedulableReason::InsufficientVram {
                    node_id: node.node_id.clone(),
                    required_bytes: task.constraints.hard.min_vram_bytes,
                    available_bytes: node.available_vram_bytes,
                });
                continue;
            }

            // 7. Required ComputeBackend
            if let Some(req_backend) = task.constraints.hard.required_backend {
                if !node.supported_backends.contains(&req_backend) {
                    last_rejection_reason = Some(UnschedulableReason::MissingBackend);
                    continue;
                }
            }

            // 8. Required SIMD instruction sets
            let mut simd_ok = true;
            for req_simd in &task.constraints.hard.required_simd {
                if !node
                    .supported_simd
                    .iter()
                    .any(|s| s.eq_ignore_ascii_case(req_simd))
                {
                    simd_ok = false;
                    last_rejection_reason = Some(UnschedulableReason::MissingSimd(req_simd.clone()));
                    break;
                }
            }
            if !simd_ok {
                continue;
            }

            // All hard constraints satisfied!
            feasible.push(node);
        }

        if feasible.is_empty() {
            Err(last_rejection_reason.unwrap_or(UnschedulableReason::NoFeasibleNodes))
        } else {
            Ok(feasible)
        }
    }

    /// Evaluates all feasible nodes using `RooflineCostModel` and selects the optimal node.
    pub fn schedule_task(
        &self,
        task: &TaskNode,
        feasible_nodes: &[&NodeCandidateProfile],
        local_node_id: &NodeId,
        input_transfer_bytes: u64,
    ) -> Result<PlacementDecision, UnschedulableReason> {
        if feasible_nodes.is_empty() {
            return Err(UnschedulableReason::NoFeasibleNodes);
        }

        let mut best_decision: Option<PlacementDecision> = None;

        for &node in feasible_nodes {
            let is_local = node.node_id == *local_node_id;
            let link = node.get_link_to(local_node_id);

            let cost = RooflineCostModel::estimate_cost(
                task,
                node,
                is_local,
                input_transfer_bytes,
                &link,
            );

            // Select best backend on candidate node
            let backend = if let Some(req) = task.constraints.hard.required_backend {
                req
            } else if let Some(&first_accel) = node
                .supported_backends
                .iter()
                .find(|b| !matches!(b, ComputeBackend::CpuSimd))
            {
                first_accel
            } else {
                ComputeBackend::CpuSimd
            };

            let reason = if is_local {
                format!(
                    "Local node selected: execution time {:.2} ms, transfer overhead 0 ms",
                    cost.roofline_execution_ms
                )
            } else {
                format!(
                    "Remote node {:?} selected: execution {:.2} ms + transfer {:.2} ms (total score: {:.2})",
                    node.node_id, cost.roofline_execution_ms, cost.transfer_time_ms, cost.total_cost_score
                )
            };

            let decision = PlacementDecision {
                task_id: task.task_id.clone(),
                assigned_node_id: node.node_id.clone(),
                selected_backend: backend,
                cost_breakdown: cost,
                is_local_execution: is_local,
                reason,
            };

            match &best_decision {
                None => {
                    best_decision = Some(decision);
                }
                Some(current_best) => {
                    if decision.cost_breakdown.total_cost_score
                        < current_best.cost_breakdown.total_cost_score
                    {
                        best_decision = Some(decision);
                    }
                }
            }
        }

        best_decision.ok_or(UnschedulableReason::NoFeasibleNodes)
    }

    /// Compiles and schedules a complete `JobDag` across cluster nodes.
    /// Operates in `< 10 ms` for 100 tasks.
    pub fn schedule_dag(
        &self,
        dag: &JobDag,
        cluster: &[NodeCandidateProfile],
        local_node_id: &NodeId,
    ) -> Result<PlacementPlan, SchedulingError> {
        let t_start = Instant::now();

        if cluster.is_empty() {
            return Err(SchedulingError::NoNodesInCluster);
        }

        // 1. Validate DAG and obtain deterministic topological order
        let topo_order = dag.topological_sort()?;

        let mut assignments: HashMap<String, PlacementDecision> = HashMap::with_capacity(topo_order.len());
        let mut total_makespan_ms: f64 = 0.0;

        // 2. Schedule each task in topological order
        for task_id in &topo_order {
            let task = dag
                .task(task_id)
                .ok_or_else(|| SchedulingError::TaskNotFound(task_id.clone()))?;

            // Compute incoming data transfer bytes from parent tasks
            let mut input_transfer_bytes = 0u64;
            for incoming_edge in dag.incoming_edges_for(task_id) {
                // If parent task is placed on a different node, data must be transferred
                if let Some(parent_placement) = assignments.get(&incoming_edge.from_task_id) {
                    if parent_placement.assigned_node_id != *local_node_id {
                        input_transfer_bytes = input_transfer_bytes.saturating_add(incoming_edge.data_dependency_bytes);
                    }
                } else {
                    input_transfer_bytes = input_transfer_bytes.saturating_add(incoming_edge.data_dependency_bytes);
                }
            }

            // Filter feasible candidate nodes
            let feasible = self
                .filter_feasible_nodes(task, cluster, local_node_id)
                .map_err(|reason| SchedulingError::UnschedulableTask {
                    task_id: task_id.clone(),
                    reason,
                })?;

            // Schedule task optimally using Roofline cost model
            let decision = self
                .schedule_task(task, &feasible, local_node_id, input_transfer_bytes)
                .map_err(|reason| SchedulingError::UnschedulableTask {
                    task_id: task_id.clone(),
                    reason,
                })?;

            total_makespan_ms += decision.cost_breakdown.roofline_execution_ms
                + decision.cost_breakdown.transfer_time_ms;

            assignments.insert(task_id.clone(), decision);
        }

        let solver_duration_micros = t_start.elapsed().as_micros() as u64;

        Ok(PlacementPlan {
            job_id: dag.job_id.clone(),
            assignments,
            topological_task_order: topo_order,
            total_estimated_makespan_ms: total_makespan_ms,
            solver_duration_micros,
        })
    }
}
