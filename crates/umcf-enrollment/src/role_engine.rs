//! Deterministic node role state machine and transitions across Master, Worker, Hybrid, and Solo modes.
//! Strictly enforces `#![forbid(unsafe_code)]` and zero-placeholder invariants.

use crate::error::EnrollmentError;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Dynamic runtime operating role of a node within the Universal Mobile Compute Fabric.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NodeRoleState {
    /// Active cluster coordinator orchestrating scheduling, heartbeats, and membership.
    Master,
    /// Dedicated compute/inference executor without cluster coordinator authority.
    Worker,
    /// Dual-mode operating node simultaneously acting as coordinator and compute executor.
    Hybrid,
    /// Node currently seeking quorum in an active election term.
    Candidate,
    /// Isolated single node operating autonomously as both coordinator and worker.
    AutonomousSolo,
}

impl fmt::Display for NodeRoleState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Master => write!(f, "Master"),
            Self::Worker => write!(f, "Worker"),
            Self::Hybrid => write!(f, "Hybrid"),
            Self::Candidate => write!(f, "Candidate"),
            Self::AutonomousSolo => write!(f, "AutonomousSolo"),
        }
    }
}

/// Recorded audit entry for a verified role transition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoleTransitionEvent {
    /// Previous role state.
    pub from: NodeRoleState,
    /// Target role state.
    pub to: NodeRoleState,
    /// Operational reason or trigger for the transition.
    pub reason: String,
    /// Timestamp when transition took effect.
    pub timestamp_epoch_ms: u64,
}

/// Deterministic state machine governing dynamic role transitions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoleStateMachine {
    current_state: NodeRoleState,
    history: Vec<RoleTransitionEvent>,
}

impl Default for RoleStateMachine {
    fn default() -> Self {
        Self::new(NodeRoleState::AutonomousSolo)
    }
}

impl RoleStateMachine {
    /// Creates a new state machine initialized to the specified starting role.
    #[must_use]
    pub fn new(initial_state: NodeRoleState) -> Self {
        Self {
            current_state: initial_state,
            history: Vec::new(),
        }
    }

    /// Returns the active role state of the node.
    #[must_use]
    pub const fn current_state(&self) -> NodeRoleState {
        self.current_state
    }

    /// Returns the audit history of all completed role transitions.
    #[must_use]
    pub fn history(&self) -> &[RoleTransitionEvent] {
        &self.history
    }

    /// Evaluates whether a transition from current state to the target state is structurally valid.
    #[must_use]
    pub fn can_transition(&self, target: NodeRoleState) -> bool {
        if self.current_state == target {
            return true; // No-op transition is permitted
        }

        match (self.current_state, target) {
            // AutonomousSolo can transition to Worker (on cluster join), Candidate (on election), or Hybrid
            (NodeRoleState::AutonomousSolo, NodeRoleState::Worker)
            | (NodeRoleState::AutonomousSolo, NodeRoleState::Candidate)
            | (NodeRoleState::AutonomousSolo, NodeRoleState::Hybrid)
            | (NodeRoleState::AutonomousSolo, NodeRoleState::Master) => true,

            // Candidate can transition to Master/Hybrid (election won), Worker (election lost), or Solo (split)
            (NodeRoleState::Candidate, NodeRoleState::Master)
            | (NodeRoleState::Candidate, NodeRoleState::Hybrid)
            | (NodeRoleState::Candidate, NodeRoleState::Worker)
            | (NodeRoleState::Candidate, NodeRoleState::AutonomousSolo) => true,

            // Master can transition to Worker (step down), Hybrid (enable compute), or Solo (partition)
            (NodeRoleState::Master, NodeRoleState::Worker)
            | (NodeRoleState::Master, NodeRoleState::Hybrid)
            | (NodeRoleState::Master, NodeRoleState::AutonomousSolo) => true,

            // Hybrid can transition to Master (disable local compute), Worker (step down coordinator), or Solo
            (NodeRoleState::Hybrid, NodeRoleState::Master)
            | (NodeRoleState::Hybrid, NodeRoleState::Worker)
            | (NodeRoleState::Hybrid, NodeRoleState::AutonomousSolo) => true,

            // Worker can transition to Candidate (timeout), Master (leader transfer), Hybrid, or Solo (partition)
            (NodeRoleState::Worker, NodeRoleState::Candidate)
            | (NodeRoleState::Worker, NodeRoleState::Master)
            | (NodeRoleState::Worker, NodeRoleState::Hybrid)
            | (NodeRoleState::Worker, NodeRoleState::AutonomousSolo) => true,

            // Other transitions are invalid
            _ => false,
        }
    }

    /// Attempts to transition the node into a new role state.
    /// Returns `Ok(())` on success, or `Err(EnrollmentError::InvalidRoleTransition)` if invalid.
    pub fn transition_to(
        &mut self,
        target: NodeRoleState,
        reason: impl Into<String>,
        timestamp_epoch_ms: u64,
    ) -> Result<(), EnrollmentError> {
        let reason_str = reason.into();

        if !self.can_transition(target) {
            return Err(EnrollmentError::InvalidRoleTransition {
                from: self.current_state.to_string(),
                to: target.to_string(),
                reason: reason_str,
            });
        }

        let event = RoleTransitionEvent {
            from: self.current_state,
            to: target,
            reason: reason_str,
            timestamp_epoch_ms,
        };

        self.current_state = target;
        self.history.push(event);
        Ok(())
    }

    /// Returns `true` if the current node possesses cluster coordination authority.
    #[must_use]
    pub const fn is_coordinator(&self) -> bool {
        matches!(
            self.current_state,
            NodeRoleState::Master | NodeRoleState::Hybrid | NodeRoleState::AutonomousSolo
        )
    }

    /// Returns `true` if the current node can execute compute/inference workloads.
    #[must_use]
    pub const fn can_execute_compute(&self) -> bool {
        matches!(
            self.current_state,
            NodeRoleState::Worker | NodeRoleState::Hybrid | NodeRoleState::AutonomousSolo
        )
    }
}
