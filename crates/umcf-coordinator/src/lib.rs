//! Master/Coordinator cluster registry, heartbeat lease management, and single-use enrollment verification.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use umcf_core::{
    CoreError, EnrollmentToken, HardwareProfile, NodeId, NodeRole, ResourceConsentPolicy,
};

/// Record of an enrolled node inside the coordinator registry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegisteredNode {
    pub node_id: NodeId,
    pub role: NodeRole,
    pub hardware: HardwareProfile,
    pub consent: ResourceConsentPolicy,
    pub last_heartbeat_unix_secs: u64,
    pub lease_expires_unix_secs: u64,
}

impl RegisteredNode {
    #[must_use]
    pub const fn is_lease_active(&self, now_unix_secs: u64) -> bool {
        now_unix_secs <= self.lease_expires_unix_secs
    }
}

/// Manages cluster membership, single-use QR/CLI enrollment tokens, and node leases.
#[derive(Debug, Clone)]
pub struct ClusterCoordinator {
    cluster_id: String,
    coordinator_node_id: NodeId,
    lease_duration_secs: u64,
    enrollment_secret: Vec<u8>,
    redeemed_tokens: BTreeSet<String>,
    nodes: BTreeMap<NodeId, RegisteredNode>,
}

impl ClusterCoordinator {
    #[must_use]
    pub fn new(
        cluster_id: impl Into<String>,
        coordinator_node_id: NodeId,
        lease_duration_secs: u64,
        enrollment_secret: Vec<u8>,
    ) -> Self {
        Self {
            cluster_id: cluster_id.into(),
            coordinator_node_id,
            lease_duration_secs: lease_duration_secs.max(5),
            enrollment_secret,
            redeemed_tokens: BTreeSet::new(),
            nodes: BTreeMap::new(),
        }
    }

    #[must_use]
    pub fn cluster_id(&self) -> &str {
        &self.cluster_id
    }

    #[must_use]
    pub fn coordinator_node_id(&self) -> &NodeId {
        &self.coordinator_node_id
    }

    /// Issues a short-lived single-use enrollment token.
    pub fn issue_enrollment_token(
        &self,
        token_id: impl Into<String>,
        now_unix_secs: u64,
        ttl_secs: u64,
    ) -> Result<EnrollmentToken, CoordinatorError> {
        EnrollmentToken::issue(
            token_id,
            self.coordinator_node_id.clone(),
            &self.cluster_id,
            now_unix_secs,
            ttl_secs,
            &self.enrollment_secret,
        )
        .map_err(CoordinatorError::Core)
    }

    /// Enrolls a node using a single-use token, verifying signature, expiration, cluster match, and replay protection.
    pub fn enroll_node(
        &mut self,
        token: &EnrollmentToken,
        role: NodeRole,
        hardware: HardwareProfile,
        consent: ResourceConsentPolicy,
        now_unix_secs: u64,
    ) -> Result<&RegisteredNode, CoordinatorError> {
        if token.cluster_id != self.cluster_id {
            return Err(CoordinatorError::ClusterIdMismatch {
                expected: self.cluster_id.clone(),
                actual: token.cluster_id.clone(),
            });
        }
        if self.redeemed_tokens.contains(&token.token_id) {
            return Err(CoordinatorError::TokenAlreadyRedeemed(
                token.token_id.clone(),
            ));
        }
        token
            .verify(now_unix_secs, &self.enrollment_secret)
            .map_err(CoordinatorError::Core)?;
        consent.validate().map_err(CoordinatorError::Core)?;

        self.redeemed_tokens.insert(token.token_id.clone());
        let node_id = hardware.node_id.clone();
        let entry = RegisteredNode {
            node_id: node_id.clone(),
            role,
            hardware,
            consent,
            last_heartbeat_unix_secs: now_unix_secs,
            lease_expires_unix_secs: now_unix_secs + self.lease_duration_secs,
        };
        self.nodes.insert(node_id.clone(), entry);
        self.nodes
            .get(&node_id)
            .ok_or(CoordinatorError::NodeNotFound(node_id))
    }

    /// Refreshes a node's heartbeat lease.
    pub fn record_heartbeat(
        &mut self,
        node_id: &NodeId,
        now_unix_secs: u64,
    ) -> Result<u64, CoordinatorError> {
        let node = self
            .nodes
            .get_mut(node_id)
            .ok_or_else(|| CoordinatorError::NodeNotFound(node_id.clone()))?;
        node.last_heartbeat_unix_secs = now_unix_secs;
        node.lease_expires_unix_secs = now_unix_secs + self.lease_duration_secs;
        Ok(node.lease_expires_unix_secs)
    }

    /// Returns all nodes with an active lease at `now_unix_secs`.
    #[must_use]
    pub fn active_nodes(&self, now_unix_secs: u64) -> Vec<&RegisteredNode> {
        self.nodes
            .values()
            .filter(|n| n.is_lease_active(now_unix_secs))
            .collect()
    }

    /// Evicts nodes whose heartbeat lease has expired.
    pub fn evict_expired_nodes(&mut self, now_unix_secs: u64) -> Vec<NodeId> {
        let expired: Vec<NodeId> = self
            .nodes
            .iter()
            .filter(|(_, n)| !n.is_lease_active(now_unix_secs))
            .map(|(id, _)| id.clone())
            .collect();
        for id in &expired {
            self.nodes.remove(id);
        }
        expired
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoordinatorError {
    Core(CoreError),
    ClusterIdMismatch { expected: String, actual: String },
    TokenAlreadyRedeemed(String),
    NodeNotFound(NodeId),
}

impl fmt::Display for CoordinatorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Core(e) => write!(f, "coordinator core error: {e}"),
            Self::ClusterIdMismatch { expected, actual } => {
                write!(f, "cluster ID mismatch: expected {expected}, got {actual}")
            }
            Self::TokenAlreadyRedeemed(id) => {
                write!(f, "enrollment token '{id}' has already been redeemed")
            }
            Self::NodeNotFound(id) => write!(f, "node not found in registry: {id}"),
        }
    }
}

impl std::error::Error for CoordinatorError {}

#[cfg(test)]
mod tests {
    use super::*;
    use umcf_core::{CpuArch, PlatformKind};

    fn sample_profile(seed: &[u8]) -> HardwareProfile {
        HardwareProfile {
            node_id: NodeId::from_public_key_bytes(seed).expect("node id"),
            platform: PlatformKind::Linux,
            cpu_arch: CpuArch::X86_64,
            cpu_model: "AMD EPYC".into(),
            physical_cores: 8,
            logical_cores: 16,
            total_ram_mb: 32768,
            available_ram_mb: 24576,
            available_storage_mb: 100_000,
            network_bandwidth_mbps: 2500,
            accelerators: vec![],
        }
    }

    #[test]
    fn integration_enroll_heartbeat_replay_protection_and_eviction() {
        let master_id =
            NodeId::from_public_key_bytes(b"master-node-pubkey-0123456789").expect("id");
        let mut coord = ClusterCoordinator::new(
            "umcf-prod-cluster",
            master_id,
            30,
            b"super-secret-key".to_vec(),
        );

        let token = coord
            .issue_enrollment_token("nonce-1", 1000, 120)
            .expect("token");
        let hw = sample_profile(b"worker-node-pubkey-0123456789");
        let worker_id = hw.node_id.clone();
        let consent = ResourceConsentPolicy {
            sharing_enabled: true,
            ..ResourceConsentPolicy::default()
        };

        let enrolled = coord
            .enroll_node(&token, NodeRole::Worker, hw.clone(), consent.clone(), 1010)
            .expect("enroll succeeds");
        assert_eq!(enrolled.node_id, worker_id);

        // Replay attempt with the same single-use token must fail
        let replay = coord.enroll_node(&token, NodeRole::Worker, hw, consent, 1015);
        assert!(matches!(
            replay,
            Err(CoordinatorError::TokenAlreadyRedeemed(_))
        ));

        // Heartbeat extends lease
        let new_exp = coord.record_heartbeat(&worker_id, 1035).expect("heartbeat");
        assert_eq!(new_exp, 1065);
        assert_eq!(coord.active_nodes(1060).len(), 1);

        // Evict after lease expiry
        let evicted = coord.evict_expired_nodes(1070);
        assert_eq!(evicted, vec![worker_id]);
        assert!(coord.active_nodes(1070).is_empty());
    }
}
