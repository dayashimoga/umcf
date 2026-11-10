//! Capability-based Role-Based Access Control (`RbacPolicyEngine`) for UMCF.
//! Strictly enforces `#![forbid(unsafe_code)]` and default-deny invariants.

use crate::error::EnrollmentError;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fmt;

/// Operating security role assigned to a node within a cluster.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Role {
    /// Full administrative authority over cluster membership, policy, and workloads.
    ClusterOwner,
    /// Orchestrates DAG scheduling, peer heartbeats, and certificate issuance/revocation.
    MasterCoordinator,
    /// Operates concurrently as both a task scheduler and a compute worker.
    HybridNode,
    /// Executes compute, AI inference, and media tasks assigned by coordinators.
    ComputeWorker,
    /// Contributes persistent distributed block and blob storage capacity.
    StorageProvider,
    /// Read-only telemetry and cluster topology observer without compute execution rights.
    ObserverGuest,
}

impl fmt::Display for Role {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ClusterOwner => write!(f, "ClusterOwner"),
            Self::MasterCoordinator => write!(f, "MasterCoordinator"),
            Self::HybridNode => write!(f, "HybridNode"),
            Self::ComputeWorker => write!(f, "ComputeWorker"),
            Self::StorageProvider => write!(f, "StorageProvider"),
            Self::ObserverGuest => write!(f, "ObserverGuest"),
        }
    }
}

/// Granular capability permissions governing mesh operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Permission {
    /// Submit new compute jobs, DAGs, or pipelines into the mesh.
    SubmitJob,
    /// Schedule and assign tasks onto worker nodes.
    ScheduleTask,
    /// Read cluster health, topology, node capabilities, and metrics.
    ReadClusterState,
    /// Issue QR enrollment tokens and approve new candidate node join requests.
    EnrollNode,
    /// Revoke node certificates and publish Certificate Revocation Lists (CRLs).
    RevokeNode,
    /// Submit and execute private on-device LLM inference tasks.
    AccessPrivateAi,
    /// Initiate or receive low-latency remote desktop and game streaming sessions.
    StreamDesktop,
    /// Read, write, and replicate chunks in the distributed storage substrate.
    ReadWriteStorage,
}

impl fmt::Display for Permission {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SubmitJob => write!(f, "SubmitJob"),
            Self::ScheduleTask => write!(f, "ScheduleTask"),
            Self::ReadClusterState => write!(f, "ReadClusterState"),
            Self::EnrollNode => write!(f, "EnrollNode"),
            Self::RevokeNode => write!(f, "RevokeNode"),
            Self::AccessPrivateAi => write!(f, "AccessPrivateAi"),
            Self::StreamDesktop => write!(f, "StreamDesktop"),
            Self::ReadWriteStorage => write!(f, "ReadWriteStorage"),
        }
    }
}

/// Target resource or boundary scope for an authorized operation.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ResourceScope {
    /// Global cluster-wide scope.
    Global,
    /// Bound to a specific cluster ID.
    Cluster(String),
    /// Bound to a specific Job ID.
    Job(String),
    /// Bound to a specific Task ID.
    Task(String),
    /// Bound to a specific Workload or model name.
    Workload(String),
}

impl fmt::Display for ResourceScope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Global => write!(f, "global"),
            Self::Cluster(id) => write!(f, "cluster:{id}"),
            Self::Job(id) => write!(f, "job:{id}"),
            Self::Task(id) => write!(f, "task:{id}"),
            Self::Workload(name) => write!(f, "workload:{name}"),
        }
    }
}

/// Authoritative RBAC policy engine enforcing least-privilege default-deny authorization.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RbacPolicyEngine {
    role_permissions: HashMap<Role, HashSet<Permission>>,
    actor_overrides: HashMap<String, HashSet<Permission>>,
}

impl Default for RbacPolicyEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl RbacPolicyEngine {
    /// Creates a new policy engine with standard default-deny permission matrices.
    #[must_use]
    pub fn new() -> Self {
        let mut role_permissions = HashMap::new();

        // 1. ClusterOwner has all 8 permissions
        let owner_perms: HashSet<Permission> = [
            Permission::SubmitJob,
            Permission::ScheduleTask,
            Permission::ReadClusterState,
            Permission::EnrollNode,
            Permission::RevokeNode,
            Permission::AccessPrivateAi,
            Permission::StreamDesktop,
            Permission::ReadWriteStorage,
        ]
        .into_iter()
        .collect();
        role_permissions.insert(Role::ClusterOwner, owner_perms);

        // 2. MasterCoordinator manages cluster orchestration and scheduling
        let master_perms: HashSet<Permission> = [
            Permission::ScheduleTask,
            Permission::ReadClusterState,
            Permission::EnrollNode,
            Permission::RevokeNode,
            Permission::AccessPrivateAi,
            Permission::ReadWriteStorage,
        ]
        .into_iter()
        .collect();
        role_permissions.insert(Role::MasterCoordinator, master_perms);

        // 3. HybridNode operates as coordinator and worker
        let hybrid_perms: HashSet<Permission> = [
            Permission::SubmitJob,
            Permission::ScheduleTask,
            Permission::ReadClusterState,
            Permission::AccessPrivateAi,
            Permission::StreamDesktop,
            Permission::ReadWriteStorage,
        ]
        .into_iter()
        .collect();
        role_permissions.insert(Role::HybridNode, hybrid_perms);

        // 4. ComputeWorker executes workloads and reads state
        let worker_perms: HashSet<Permission> = [
            Permission::ReadClusterState,
            Permission::AccessPrivateAi,
        ]
        .into_iter()
        .collect();
        role_permissions.insert(Role::ComputeWorker, worker_perms);

        // 5. StorageProvider contributes storage substrate
        let storage_perms: HashSet<Permission> = [
            Permission::ReadClusterState,
            Permission::ReadWriteStorage,
        ]
        .into_iter()
        .collect();
        role_permissions.insert(Role::StorageProvider, storage_perms);

        // 6. ObserverGuest read-only telemetry
        let guest_perms: HashSet<Permission> = [Permission::ReadClusterState]
            .into_iter()
            .collect();
        role_permissions.insert(Role::ObserverGuest, guest_perms);

        Self {
            role_permissions,
            actor_overrides: HashMap::new(),
        }
    }

    /// Evaluates whether an actor with an assigned role is authorized for a permission.
    /// Strictly default-deny: returns `Err(EnrollmentError::RbacDenied)` if not explicitly granted.
    pub fn authorize(
        &self,
        actor_did: &str,
        role: Role,
        permission: Permission,
        scope: &ResourceScope,
    ) -> Result<(), EnrollmentError> {
        if self.has_permission(actor_did, role, permission) {
            Ok(())
        } else {
            Err(EnrollmentError::RbacDenied {
                actor_did: actor_did.to_string(),
                role: role.to_string(),
                permission: permission.to_string(),
                resource_scope: scope.to_string(),
            })
        }
    }

    /// Checks whether the actor/role combination possesses the requested permission.
    #[must_use]
    pub fn has_permission(&self, actor_did: &str, role: Role, permission: Permission) -> bool {
        // Check per-actor specific override first
        if let Some(overrides) = self.actor_overrides.get(actor_did) {
            if overrides.contains(&permission) {
                return true;
            }
        }

        // Check role permission set
        if let Some(perms) = self.role_permissions.get(&role) {
            perms.contains(&permission)
        } else {
            false
        }
    }

    /// Grants an actor-specific permission override.
    pub fn grant_actor_permission(&mut self, actor_did: impl Into<String>, permission: Permission) {
        self.actor_overrides
            .entry(actor_did.into())
            .or_default()
            .insert(permission);
    }

    /// Revokes an actor-specific permission override.
    pub fn revoke_actor_permission(&mut self, actor_did: &str, permission: Permission) {
        if let Some(overrides) = self.actor_overrides.get_mut(actor_did) {
            overrides.remove(&permission);
        }
    }

    /// Returns the complete list of permissions granted to a given role.
    #[must_use]
    pub fn get_role_permissions(&self, role: Role) -> Vec<Permission> {
        let mut perms: Vec<Permission> = self
            .role_permissions
            .get(&role)
            .map(|s| s.iter().copied().collect())
            .unwrap_or_default();
        perms.sort_by_key(|p| *p as u8);
        perms
    }
}
