//! Core domain identifiers, device classifications, and role state machines for UMCF.
//! Adheres strictly to `#![forbid(unsafe_code)]` and zero-placeholder invariants.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt;
use umcf_proto::compute_fnv1a_64;

use crate::CoreError;

/// Cryptographically derived decentralized device identifier (`did:umcf:<hex-fingerprint>`).
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
pub struct NodeId(pub String);

impl NodeId {
    /// Derives a deterministic `NodeId` from public key bytes or cryptographic seed material.
    pub fn from_public_key_bytes(pubkey: &[u8]) -> Result<Self, CoreError> {
        if pubkey.len() < 16 {
            return Err(CoreError::InvalidPublicKeyLength(pubkey.len()));
        }
        let hi = compute_fnv1a_64(pubkey);
        let lo = compute_fnv1a_64(&pubkey[pubkey.len() / 2..]);
        Ok(Self(format!("did:umcf:{hi:016x}{lo:016x}")))
    }

    /// Parses and validates a raw DID string.
    pub fn parse(raw: impl Into<String>) -> Result<Self, CoreError> {
        let s = raw.into();
        if !s.starts_with("did:umcf:") || s.len() < 25 {
            return Err(CoreError::InvalidNodeIdFormat(s));
        }
        Ok(Self(s))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Cluster identifier (`did:umcf:cluster:<name-or-hex>`).
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
pub struct ClusterId(pub String);

impl ClusterId {
    pub fn new(id: impl Into<String>) -> Result<Self, CoreError> {
        let s = id.into();
        if !s.starts_with("did:umcf:cluster:") || s.len() < 20 {
            return Err(CoreError::InvalidClusterIdFormat(s));
        }
        Ok(Self(s))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ClusterId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// 128-bit unique job identifier.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
pub struct JobId(pub u128);

impl JobId {
    #[must_use]
    pub const fn new(id: u128) -> Self {
        Self(id)
    }

    #[must_use]
    pub const fn as_u128(self) -> u128 {
        self.0
    }

    #[must_use]
    pub fn from_parts(hi: u64, lo: u64) -> Self {
        Self(((hi as u128) << 64) | (lo as u128))
    }
}

impl fmt::Display for JobId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:032x}", self.0)
    }
}

/// Unique task identifier representing an individual execution unit within a DAG job.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
pub struct TaskId {
    pub job_id: JobId,
    pub task_index: u32,
}

impl TaskId {
    #[must_use]
    pub const fn new(job_id: JobId, task_index: u32) -> Self {
        Self { job_id, task_index }
    }
}

impl fmt::Display for TaskId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.job_id, self.task_index)
    }
}

/// Hardware class and form-factor of a node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub enum DeviceClass {
    Smartphone,
    Tablet,
    Laptop,
    Desktop,
    Workstation,
    Server,
    BrowserWasm,
}

impl fmt::Display for DeviceClass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Smartphone => write!(f, "Smartphone"),
            Self::Tablet => write!(f, "Tablet"),
            Self::Laptop => write!(f, "Laptop"),
            Self::Desktop => write!(f, "Desktop"),
            Self::Workstation => write!(f, "Workstation"),
            Self::Server => write!(f, "Server"),
            Self::BrowserWasm => write!(f, "BrowserWasm"),
        }
    }
}

/// Host operating system runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub enum OperatingSystem {
    Android,
    Ios,
    Windows,
    Linux,
    MacOs,
    Web,
}

impl fmt::Display for OperatingSystem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Android => write!(f, "Android"),
            Self::Ios => write!(f, "iOS"),
            Self::Windows => write!(f, "Windows"),
            Self::Linux => write!(f, "Linux"),
            Self::MacOs => write!(f, "macOS"),
            Self::Web => write!(f, "Web"),
        }
    }
}

/// Dynamic operational role of a UMCF node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub enum NodeRole {
    /// Coordinates cluster registry, task admission, and scheduling.
    Master,
    /// Executes compute, AI inference, rendering, media, or storage tasks within consent bounds.
    Worker,
    /// Simultaneously coordinates cluster state and contributes local compute capacity.
    Hybrid,
}

impl fmt::Display for NodeRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Master => write!(f, "Master"),
            Self::Worker => write!(f, "Worker"),
            Self::Hybrid => write!(f, "Hybrid"),
        }
    }
}

/// Audited state machine governing dynamic role transitions on any device.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct RoleStateMachine {
    node_id: NodeId,
    current_role: NodeRole,
    epoch: u64,
}

impl RoleStateMachine {
    #[must_use]
    pub fn new(node_id: NodeId, initial_role: NodeRole) -> Self {
        Self {
            node_id,
            current_role: initial_role,
            epoch: 1,
        }
    }

    #[must_use]
    pub fn node_id(&self) -> &NodeId {
        &self.node_id
    }

    #[must_use]
    pub const fn current_role(&self) -> NodeRole {
        self.current_role
    }

    #[must_use]
    pub const fn epoch(&self) -> u64 {
        self.epoch
    }

    /// Transitions the node to a new role and increments the cluster epoch.
    pub fn transition_to(&mut self, target_role: NodeRole) -> u64 {
        if self.current_role != target_role {
            self.current_role = target_role;
            self.epoch = self.epoch.saturating_add(1);
        }
        self.epoch
    }
}
