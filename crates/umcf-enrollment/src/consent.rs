//! Explicit user-controlled Resource-Sharing Consent manifests (`ResourceConsentPolicy`)
//! and local sovereignty enforcement (`ConsentEnforcer`).
//! Strictly enforces `#![forbid(unsafe_code)]` and zero-placeholder invariants.

use crate::error::EnrollmentError;
use crate::rbac::{Permission, RbacPolicyEngine, ResourceScope, Role};
use ed25519_dalek::VerifyingKey;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fmt;
use std::sync::{Arc, RwLock};
use umcf_core::{ByteSize, NodeId, ThermalState};
use umcf_identity::{NodeKeypair, DOMAIN_SEPARATION_PREFIX};

/// Domain separation context for signing resource consent policies.
pub const CONSENT_POLICY_DOMAIN: &[u8] = b"UMCF-CONSENT-v1:";

/// Category of compute, AI, media, or storage workload.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WorkloadKind {
    /// General CPU batch compute jobs.
    CpuBatch,
    /// GPU-accelerated LLM/tensor inference shards.
    GpuInference,
    /// GPU 3D graphics, rendering, and video transcode.
    GpuRendering,
    /// NPU-accelerated neural network execution.
    NpuInference,
    /// Storage substrate chunk relay and replication.
    StorageRelay,
    /// Ultra-low latency remote desktop or game streaming.
    RemoteStreaming,
}

impl fmt::Display for WorkloadKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CpuBatch => write!(f, "CpuBatch"),
            Self::GpuInference => write!(f, "GpuInference"),
            Self::GpuRendering => write!(f, "GpuRendering"),
            Self::NpuInference => write!(f, "NpuInference"),
            Self::StorageRelay => write!(f, "StorageRelay"),
            Self::RemoteStreaming => write!(f, "RemoteStreaming"),
        }
    }
}

/// Quiet hours schedule during which remote compute execution is restricted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuietHoursSchedule {
    /// UTC start hour (0..=23, inclusive).
    pub start_hour_utc: u8,
    /// UTC end hour (0..=23, exclusive).
    pub end_hour_utc: u8,
}

impl QuietHoursSchedule {
    /// Creates a validated quiet hours window.
    pub fn new(start_hour_utc: u8, end_hour_utc: u8) -> Result<Self, EnrollmentError> {
        if start_hour_utc > 23 || end_hour_utc > 23 {
            return Err(EnrollmentError::ConsentViolation {
                reason: format!(
                    "invalid UTC quiet hours ({start_hour_utc}-{end_hour_utc}); must be 0..23"
                ),
            });
        }
        Ok(Self {
            start_hour_utc,
            end_hour_utc,
        })
    }

    /// Evaluates whether the current UTC hour falls within the quiet hours window.
    #[must_use]
    pub const fn is_quiet_hour(&self, current_hour_utc: u8) -> bool {
        if self.start_hour_utc <= self.end_hour_utc {
            current_hour_utc >= self.start_hour_utc && current_hour_utc < self.end_hour_utc
        } else {
            // Window crosses midnight (e.g., 22:00 to 06:00)
            current_hour_utc >= self.start_hour_utc || current_hour_utc < self.end_hour_utc
        }
    }
}

/// Signed resource consent policy defining explicit boundaries under which a device contributes compute.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceConsentPolicy {
    /// W3C did:key of the local device owner.
    pub owner_did: String,
    /// Unique identifier of the local node.
    pub node_id: NodeId,
    /// Maximum allowed CPU core utilization percentage (0..=100).
    pub max_cpu_percent: u8,
    /// Maximum RAM allocated for remote mesh tasks.
    pub max_ram_bytes: ByteSize,
    /// Maximum GPU VRAM allocated for remote mesh tasks.
    pub max_vram_bytes: ByteSize,
    /// Maximum local storage space allocated for remote caching/shards.
    pub max_storage_bytes: ByteSize,
    /// Set of permitted workload categories.
    pub allowed_workload_kinds: HashSet<WorkloadKind>,
    /// Whether execution is restricted to AC wall charging.
    pub require_charging: bool,
    /// Battery percent floor below which all remote workloads are aborted.
    pub min_battery_percent: u8,
    /// Optional scheduled quiet hours.
    pub quiet_hours: Option<QuietHoursSchedule>,
    /// Whitelist of caller DIDs allowed to assign workloads (empty = open to cluster).
    pub allowed_requester_dids: Vec<String>,
    /// Monotonically increasing generation sequence counter.
    pub policy_generation: u64,
    /// Timestamp when this policy was signed and updated.
    pub updated_at_epoch_ms: u64,
    /// Cryptographic Ed25519 signature by the local device owner keypair.
    #[serde(with = "signature_hex")]
    pub owner_signature: [u8; 64],
}

impl ResourceConsentPolicy {
    /// Builds and cryptographically signs a `ResourceConsentPolicy` using the local device keypair.
    #[allow(clippy::too_many_arguments)]
    pub fn sign_new(
        owner: &NodeKeypair,
        node_id: NodeId,
        max_cpu_percent: u8,
        max_ram_bytes: ByteSize,
        max_vram_bytes: ByteSize,
        max_storage_bytes: ByteSize,
        allowed_workload_kinds: HashSet<WorkloadKind>,
        require_charging: bool,
        min_battery_percent: u8,
        quiet_hours: Option<QuietHoursSchedule>,
        allowed_requester_dids: Vec<String>,
        policy_generation: u64,
        updated_at_epoch_ms: u64,
    ) -> Result<Self, EnrollmentError> {
        let cpu_capped = max_cpu_percent.min(100);
        let bat_capped = min_battery_percent.min(100);

        let mut policy = Self {
            owner_did: owner.did_key(),
            node_id,
            max_cpu_percent: cpu_capped,
            max_ram_bytes,
            max_vram_bytes,
            max_storage_bytes,
            allowed_workload_kinds,
            require_charging,
            min_battery_percent: bat_capped,
            quiet_hours,
            allowed_requester_dids,
            policy_generation,
            updated_at_epoch_ms,
            owner_signature: [0u8; 64],
        };

        let digest = policy.compute_digest();
        let sig = owner.sign(&digest);
        policy.owner_signature = sig;

        Ok(policy)
    }

    /// Computes canonical 256-bit BLAKE3 digest of the signable policy attributes.
    #[must_use]
    pub fn compute_digest(&self) -> [u8; 32] {
        let mut hasher = blake3::Hasher::new();
        hasher.update(DOMAIN_SEPARATION_PREFIX);
        hasher.update(CONSENT_POLICY_DOMAIN);
        hasher.update(self.owner_did.as_bytes());
        hasher.update(self.node_id.as_str().as_bytes());
        hasher.update(&[self.max_cpu_percent]);
        hasher.update(&self.max_ram_bytes.as_bytes().to_be_bytes());
        hasher.update(&self.max_vram_bytes.as_bytes().to_be_bytes());
        hasher.update(&self.max_storage_bytes.as_bytes().to_be_bytes());

        let mut kinds: Vec<u8> = self
            .allowed_workload_kinds
            .iter()
            .map(|k| *k as u8)
            .collect();
        kinds.sort_unstable();
        hasher.update(&kinds);

        hasher.update(&[u8::from(self.require_charging)]);
        hasher.update(&[self.min_battery_percent]);

        if let Some(qh) = &self.quiet_hours {
            hasher.update(&[1u8, qh.start_hour_utc, qh.end_hour_utc]);
        } else {
            hasher.update(&[0u8]);
        }

        for did in &self.allowed_requester_dids {
            hasher.update(did.as_bytes());
        }

        hasher.update(&self.policy_generation.to_be_bytes());
        hasher.update(&self.updated_at_epoch_ms.to_be_bytes());

        *hasher.finalize().as_bytes()
    }

    /// Verifies the cryptographic Ed25519 signature on this policy against the owner public key.
    pub fn verify_signature(&self, owner_public_key: &[u8; 32]) -> Result<(), EnrollmentError> {
        let verifying_key = VerifyingKey::from_bytes(owner_public_key).map_err(|_| {
            EnrollmentError::ForgedConsentPolicy {
                owner_did: self.owner_did.clone(),
            }
        })?;

        let digest = self.compute_digest();
        NodeKeypair::verify_with_key(&verifying_key, &digest, &self.owner_signature).map_err(|_| {
            EnrollmentError::ForgedConsentPolicy {
                owner_did: self.owner_did.clone(),
            }
        })
    }
}

/// Audit event emitted whenever a device owner updates their resource consent policy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConsentUpdatedEvent {
    /// Node whose policy was modified.
    pub node_id: NodeId,
    /// New policy generation counter.
    pub policy_generation: u64,
    /// Timestamp when policy was applied.
    pub timestamp_epoch_ms: u64,
    /// Canonical BLAKE3 digest of the newly active policy.
    pub policy_digest: [u8; 32],
    /// Owner cryptographic signature confirming the update.
    #[serde(with = "signature_hex")]
    pub owner_signature: [u8; 64],
}

/// Request to assign or execute a workload on a local node.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TaskAssignmentRequest {
    /// W3C did:key of the requesting node or user.
    pub requester_did: String,
    /// Operating role claimed by the requester.
    pub requester_role: Role,
    /// Target workload kind.
    pub workload_kind: WorkloadKind,
    /// Number of physical CPU cores requested.
    pub requested_cpu_cores: f32,
    /// RAM required for the workload.
    pub requested_ram: ByteSize,
    /// VRAM required for the workload.
    pub requested_vram: ByteSize,
    /// Persistent storage allocation required.
    pub requested_storage: ByteSize,
}

/// Instantaneous hardware and environment metrics of the local device.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeviceEnvironmentState {
    /// Current battery percent (None if AC-only desktop or server).
    pub battery_percent: Option<u8>,
    /// Whether the device is plugged into AC power.
    pub is_charging: bool,
    /// Current thermal pressure level.
    pub thermal_state: ThermalState,
    /// Current epoch timestamp in milliseconds.
    pub current_epoch_ms: u64,
    /// Current UTC hour (0..=23).
    pub current_hour_utc: u8,
}

/// Local sovereignty gate intercepting incoming task assignments and enforcing owner consent.
pub struct ConsentEnforcer {
    active_policy: Arc<RwLock<ResourceConsentPolicy>>,
    owner_public_key: [u8; 32],
    rbac: Arc<RbacPolicyEngine>,
}

impl std::fmt::Debug for ConsentEnforcer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConsentEnforcer")
            .field("owner_public_key", &hex::encode(self.owner_public_key))
            .finish_non_exhaustive()
    }
}

impl ConsentEnforcer {
    /// Creates a new consent enforcer initialized with a verified owner policy.
    pub fn new(
        initial_policy: ResourceConsentPolicy,
        owner_public_key: [u8; 32],
        rbac: Arc<RbacPolicyEngine>,
    ) -> Result<Self, EnrollmentError> {
        initial_policy.verify_signature(&owner_public_key)?;

        Ok(Self {
            active_policy: Arc::new(RwLock::new(initial_policy)),
            owner_public_key,
            rbac,
        })
    }

    /// Returns a clone of the currently active consent policy.
    pub fn active_policy(&self) -> ResourceConsentPolicy {
        self.active_policy
            .read()
            .expect("consent lock poisoned")
            .clone()
    }

    /// Updates the active policy immediately (< 10 ms), enforcing local owner signature verification.
    /// Emits a signed `ConsentUpdatedEvent`.
    pub fn update_policy(
        &self,
        new_policy: ResourceConsentPolicy,
    ) -> Result<ConsentUpdatedEvent, EnrollmentError> {
        // Invariant: foreign master or unauthorized peers cannot forge policy updates
        new_policy.verify_signature(&self.owner_public_key)?;

        let mut lock = self.active_policy.write().expect("consent lock poisoned");
        if new_policy.policy_generation <= lock.policy_generation {
            return Err(EnrollmentError::ConsentViolation {
                reason: format!(
                    "policy generation {} must be strictly greater than active generation {}",
                    new_policy.policy_generation, lock.policy_generation
                ),
            });
        }

        let event = ConsentUpdatedEvent {
            node_id: new_policy.node_id.clone(),
            policy_generation: new_policy.policy_generation,
            timestamp_epoch_ms: new_policy.updated_at_epoch_ms,
            policy_digest: new_policy.compute_digest(),
            owner_signature: new_policy.owner_signature,
        };

        *lock = new_policy;
        Ok(event)
    }

    /// Intercepts an incoming task assignment and verifies both RBAC permission and local consent policy.
    pub fn evaluate_task_assignment(
        &self,
        assignment: &TaskAssignmentRequest,
        current_env: &DeviceEnvironmentState,
    ) -> Result<(), EnrollmentError> {
        // 1. Authorize via RBAC: Requester must possess ScheduleTask or SubmitJob
        self.rbac.authorize(
            &assignment.requester_did,
            assignment.requester_role,
            Permission::ScheduleTask,
            &ResourceScope::Global,
        )?;

        let policy = self.active_policy.read().expect("consent lock poisoned");

        // 2. Requester Whitelist check (if configured)
        if !policy.allowed_requester_dids.is_empty()
            && !policy
                .allowed_requester_dids
                .contains(&assignment.requester_did)
        {
            return Err(EnrollmentError::ConsentViolation {
                reason: format!(
                    "requester {} is not present in local owner allowed_requester_dids whitelist",
                    assignment.requester_did
                ),
            });
        }

        // 3. Workload kind permission check
        if !policy
            .allowed_workload_kinds
            .contains(&assignment.workload_kind)
        {
            return Err(EnrollmentError::WorkloadNotPermitted {
                workload_kind: assignment.workload_kind.to_string(),
            });
        }

        // 4. Power & Charging check
        if policy.require_charging && !current_env.is_charging {
            return Err(EnrollmentError::ConsentViolation {
                reason: "task rejected: device is discharging and policy requires AC power"
                    .to_string(),
            });
        }

        // 5. Battery floor check
        if let Some(battery) = current_env.battery_percent {
            if battery < policy.min_battery_percent {
                return Err(EnrollmentError::ConsentViolation {
                    reason: format!(
                        "task rejected: battery level {battery}% is below configured floor of {}%",
                        policy.min_battery_percent
                    ),
                });
            }
        }

        // 6. Quiet hours check
        if let Some(qh) = &policy.quiet_hours {
            if qh.is_quiet_hour(current_env.current_hour_utc) {
                return Err(EnrollmentError::ConsentViolation {
                    reason: format!(
                        "task rejected: execution blocked during quiet hours (current hour UTC: {})",
                        current_env.current_hour_utc
                    ),
                });
            }
        }

        // 7. RAM capacity ceiling
        if assignment.requested_ram.as_bytes() > policy.max_ram_bytes.as_bytes() {
            return Err(EnrollmentError::ConsentViolation {
                reason: format!(
                    "requested RAM {} exceeds local consent limit {}",
                    assignment.requested_ram, policy.max_ram_bytes
                ),
            });
        }

        // 8. VRAM capacity ceiling
        if assignment.requested_vram.as_bytes() > policy.max_vram_bytes.as_bytes() {
            return Err(EnrollmentError::ConsentViolation {
                reason: format!(
                    "requested VRAM {} exceeds local consent limit {}",
                    assignment.requested_vram, policy.max_vram_bytes
                ),
            });
        }

        // 9. Storage capacity ceiling
        if assignment.requested_storage.as_bytes() > policy.max_storage_bytes.as_bytes() {
            return Err(EnrollmentError::ConsentViolation {
                reason: format!(
                    "requested storage {} exceeds local consent limit {}",
                    assignment.requested_storage, policy.max_storage_bytes
                ),
            });
        }

        Ok(())
    }
}

mod signature_hex {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(sig: &[u8; 64], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&super::hex::encode(sig))
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<[u8; 64], D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let bytes = super::hex::decode(&s).map_err(serde::de::Error::custom)?;
        if bytes.len() != 64 {
            return Err(serde::de::Error::custom("expected 64 bytes for signature"));
        }
        let mut out = [0u8; 64];
        out.copy_from_slice(&bytes);
        Ok(out)
    }
}

mod hex {
    pub fn encode(data: impl AsRef<[u8]>) -> String {
        data.as_ref().iter().map(|b| format!("{b:02x}")).collect()
    }

    pub fn decode(s: &str) -> Result<Vec<u8>, String> {
        if !s.len().is_multiple_of(2) {
            return Err("odd hex string length".into());
        }
        (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16).map_err(|e| e.to_string()))
            .collect()
    }
}
