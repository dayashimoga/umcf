//! `umcf-scheduler::dag`: Directed Acyclic Graph (DAG) definitions, TaskNode, TaskEdge, and PlacementConstraint.
//! Enforces `#![forbid(unsafe_code)]` and zero placeholders.

#![forbid(unsafe_code)]

use std::collections::{HashMap, VecDeque};
use umcf_core::{ComputeBackend, NodeId, NodeRole};

use crate::error::SchedulingError;

/// Classification of compute workloads scheduled across the fabric.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum WorkloadKind {
    GeneralCompute,
    LlamaLayerShard,
    MatrixMultiply,
    MediaEncode,
    WasmSandboxed,
}

/// Specification of a single task within a `JobDag`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TaskSpec {
    pub name: String,
    pub workload_kind: WorkloadKind,
    pub estimated_flops: u64,
    pub memory_bytes: u64,
    pub output_bytes: u64,
    pub is_private_ai_local_only: bool,
}

impl Default for TaskSpec {
    fn default() -> Self {
        Self {
            name: "unnamed-task".to_string(),
            workload_kind: WorkloadKind::GeneralCompute,
            estimated_flops: 1_000_000,
            memory_bytes: 10 * 1024 * 1024,
            output_bytes: 1024 * 1024,
            is_private_ai_local_only: false,
        }
    }
}

/// Hard requirements that a candidate node MUST satisfy to be feasible.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct HardRequirements {
    pub required_backend: Option<ComputeBackend>,
    pub min_ram_bytes: u64,
    pub min_vram_bytes: u64,
    pub required_simd: Vec<String>,
    pub privacy_local_only: bool,
    pub allowed_node_roles: Vec<NodeRole>,
    pub disallowed_node_ids: Vec<NodeId>,
}

impl Default for HardRequirements {
    fn default() -> Self {
        Self {
            required_backend: None,
            min_ram_bytes: 0,
            min_vram_bytes: 0,
            required_simd: Vec::new(),
            privacy_local_only: false,
            allowed_node_roles: vec![NodeRole::Worker, NodeRole::Hybrid, NodeRole::Master],
            disallowed_node_ids: Vec::new(),
        }
    }
}

/// Soft preferences used by the cost model to score eligible candidate nodes.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SoftPreferences {
    pub prefer_local: bool,
    pub prefer_high_throughput: bool,
    pub prefer_low_power: bool,
    pub data_locality_task_id: Option<String>,
}

impl Default for SoftPreferences {
    fn default() -> Self {
        Self {
            prefer_local: true,
            prefer_high_throughput: true,
            prefer_low_power: false,
            data_locality_task_id: None,
        }
    }
}

/// Combined placement constraints (hard feasibility filter + soft cost scoring).
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PlacementConstraint {
    pub hard: HardRequirements,
    pub soft: SoftPreferences,
}

/// A node in the DAG representing an executable task.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TaskNode {
    pub task_id: String,
    pub spec: TaskSpec,
    pub constraints: PlacementConstraint,
}

impl TaskNode {
    #[must_use]
    pub fn new(task_id: impl Into<String>, spec: TaskSpec, constraints: PlacementConstraint) -> Self {
        Self {
            task_id: task_id.into(),
            spec,
            constraints,
        }
    }
}

/// A directed edge representing a data dependency between two tasks in the DAG.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TaskEdge {
    pub from_task_id: String,
    pub to_task_id: String,
    pub data_dependency_bytes: u64,
}

/// Directed Acyclic Graph of tasks representing a distributed job.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct JobDag {
    pub job_id: String,
    tasks: HashMap<String, TaskNode>,
    edges: Vec<TaskEdge>,
    incoming_edges: HashMap<String, Vec<TaskEdge>>,
    outgoing_edges: HashMap<String, Vec<TaskEdge>>,
}

impl JobDag {
    /// Creates a new empty `JobDag`.
    #[must_use]
    pub fn new(job_id: impl Into<String>) -> Self {
        Self {
            job_id: job_id.into(),
            tasks: HashMap::new(),
            edges: Vec::new(),
            incoming_edges: HashMap::new(),
            outgoing_edges: HashMap::new(),
        }
    }

    /// Adds a task to the DAG.
    pub fn add_task(&mut self, task: TaskNode) -> Result<(), SchedulingError> {
        let task_id = task.task_id.clone();
        if self.tasks.insert(task_id.clone(), task).is_some() {
            return Err(SchedulingError::InvalidDag(format!(
                "Duplicate task ID '{task_id}' in JobDag"
            )));
        }
        Ok(())
    }

    /// Adds a directed data-dependency edge from `from_task_id` to `to_task_id`.
    pub fn add_edge(
        &mut self,
        from_task_id: impl Into<String>,
        to_task_id: impl Into<String>,
        data_dependency_bytes: u64,
    ) -> Result<(), SchedulingError> {
        let from = from_task_id.into();
        let to = to_task_id.into();

        if !self.tasks.contains_key(&from) {
            return Err(SchedulingError::TaskNotFound(from));
        }
        if !self.tasks.contains_key(&to) {
            return Err(SchedulingError::TaskNotFound(to));
        }
        if from == to {
            return Err(SchedulingError::CycleInDag(format!(
                "Self-loop on task '{from}'"
            )));
        }

        let edge = TaskEdge {
            from_task_id: from.clone(),
            to_task_id: to.clone(),
            data_dependency_bytes,
        };

        self.edges.push(edge.clone());
        self.outgoing_edges
            .entry(from)
            .or_default()
            .push(edge.clone());
        self.incoming_edges.entry(to).or_default().push(edge);

        Ok(())
    }

    /// Returns a reference to a task if present.
    #[must_use]
    pub fn task(&self, task_id: &str) -> Option<&TaskNode> {
        self.tasks.get(task_id)
    }

    /// Returns map of all tasks.
    #[must_use]
    pub fn tasks(&self) -> &HashMap<String, TaskNode> {
        &self.tasks
    }

    /// Returns slice of all edges.
    #[must_use]
    pub fn edges(&self) -> &[TaskEdge] {
        &self.edges
    }

    /// Returns incoming edges for a task.
    #[must_use]
    pub fn incoming_edges_for(&self, task_id: &str) -> &[TaskEdge] {
        self.incoming_edges
            .get(task_id)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Returns outgoing edges for a task.
    #[must_use]
    pub fn outgoing_edges_for(&self, task_id: &str) -> &[TaskEdge] {
        self.outgoing_edges
            .get(task_id)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Computes a deterministic topological ordering using Kahn's algorithm.
    /// Returns `SchedulingError::CycleInDag` if a cycle is detected.
    pub fn topological_sort(&self) -> Result<Vec<String>, SchedulingError> {
        if self.tasks.is_empty() {
            return Err(SchedulingError::EmptyDag);
        }

        // Calculate in-degree for each task
        let mut in_degrees: HashMap<String, usize> = HashMap::with_capacity(self.tasks.len());
        for task_id in self.tasks.keys() {
            in_degrees.insert(task_id.clone(), 0);
        }
        for edge in &self.edges {
            if let Some(deg) = in_degrees.get_mut(&edge.to_task_id) {
                *deg += 1;
            }
        }

        // Collect all tasks with zero in-degree into queue (sorted for deterministic ordering)
        let mut zero_in_degree: Vec<String> = in_degrees
            .iter()
            .filter(|(_, &deg)| deg == 0)
            .map(|(id, _)| id.clone())
            .collect();
        zero_in_degree.sort_unstable();

        let mut queue: VecDeque<String> = zero_in_degree.into();
        let mut sorted: Vec<String> = Vec::with_capacity(self.tasks.len());

        while let Some(task_id) = queue.pop_front() {
            sorted.push(task_id.clone());

            if let Some(out_edges) = self.outgoing_edges.get(&task_id) {
                let mut newly_ready: Vec<String> = Vec::new();
                for edge in out_edges {
                    if let Some(deg) = in_degrees.get_mut(&edge.to_task_id) {
                        *deg = deg.saturating_sub(1);
                        if *deg == 0 {
                            newly_ready.push(edge.to_task_id.clone());
                        }
                    }
                }
                newly_ready.sort_unstable();
                for ready_id in newly_ready {
                    queue.push_back(ready_id);
                }
            }
        }

        if sorted.len() != self.tasks.len() {
            // Find a task in a cycle
            let in_cycle = in_degrees
                .iter()
                .find(|(_, &deg)| deg > 0)
                .map(|(id, _)| id.clone())
                .unwrap_or_else(|| "unknown".to_string());
            return Err(SchedulingError::CycleInDag(in_cycle));
        }

        Ok(sorted)
    }

    /// Validates the DAG for correctness.
    pub fn validate(&self) -> Result<(), SchedulingError> {
        if self.tasks.is_empty() {
            return Err(SchedulingError::EmptyDag);
        }
        self.topological_sort()?;
        Ok(())
    }
}
