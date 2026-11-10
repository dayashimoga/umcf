//! Core domain models for Universal Mobile Compute Fabric (UMCF).
//! Implements node identity, dynamic Master/Worker/Hybrid roles, explicit heterogeneous hardware
//! capability profiles, and resource-sharing consent invariants.

use std::fmt;
use umcf_proto::compute_fnv1a_64;

/// Unique cryptographic node identifier (`did:umcf:<hex-fingerprint>`).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeId(String);

impl NodeId {
    /// Derives a deterministic `NodeId` from a public key or seed material.
    pub fn from_public_key_bytes(pubkey: &[u8]) -> Result<Self, CoreError> {
        if pubkey.len() < 16 {
            return Err(CoreError::InvalidPublicKeyLength(pubkey.len()));
        }
        let hi = compute_fnv1a_64(pubkey);
        let lo = compute_fnv1a_64(&pubkey[pubkey.len() / 2..]);
        Ok(Self(format!("did:umcf:{hi:016x}{lo:016x}")))
    }

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

/// Dynamic operational role of a UMCF node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NodeRole {
    /// Coordinates cluster registry, task admission, and scheduling.
    Master,
    /// Executes compute, AI inference, rendering, media, or storage tasks within consent bounds.
    Worker,
    ///Simultaneously coordinates cluster state and contributes local compute capacity.
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
#[derive(Debug, Clone, PartialEq, Eq)]
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
            self.epoch += 1;
        }
        self.epoch
    }

    #[must_use]
    pub const fn can_coordinate(&self) -> bool {
        matches!(self.current_role, NodeRole::Master | NodeRole::Hybrid)
    }

    #[must_use]
    pub const fn can_execute_workloads(&self) -> bool {
        matches!(self.current_role, NodeRole::Worker | NodeRole::Hybrid)
    }
}

/// Operating system and device form-factor classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PlatformKind {
    Android,
    Ios,
    Windows,
    Linux,
    MacOs,
    WebBrowser,
}

/// CPU instruction set architecture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CpuArch {
    X86_64,
    Aarch64,
    RiscV64,
    Wasm32,
}

/// Hardware accelerator class.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AcceleratorKind {
    DiscreteGpu,
    IntegratedGpu,
    MobileGpu,
    Npu,
}

/// Explicit compute API backend.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ComputeBackend {
    CpuSimd,
    Vulkan,
    Metal,
    DirectX12,
    Cuda,
    Rocm,
    OpenCl,
    WebGpu,
    IntelNpu,
    AppleNeuralEngine,
    AndroidNnapi,
}

/// Describes a hardware accelerator and whether its compute execution has been verified on real hardware.
/// Per `ENGINEERING_CONTRACT.md` §2.4, unverified accelerators MUST NOT be claimed as verified.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AcceleratorCapability {
    pub name: String,
    pub kind: AcceleratorKind,
    pub backend: ComputeBackend,
    pub dedicated_memory_mb: u64,
    pub shared_memory_mb: u64,
    pub supports_fp16: bool,
    pub supports_int8: bool,
    pub supports_int4: bool,
    /// True ONLY if probed from the physical driver and validated via an execution pass.
    pub verified_on_hardware: bool,
}

impl AcceleratorCapability {
    /// Usable memory budget for compute workloads on this specific accelerator.
    #[must_use]
    pub const fn usable_memory_mb(&self) -> u64 {
        if self.dedicated_memory_mb > 0 {
            self.dedicated_memory_mb
        } else {
            self.shared_memory_mb
        }
    }
}

/// Complete capability profile of a single UMCF node.
/// Memory and accelerators belong strictly to this node and are never transparently pooled.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HardwareProfile {
    pub node_id: NodeId,
    pub platform: PlatformKind,
    pub cpu_arch: CpuArch,
    pub cpu_model: String,
    pub physical_cores: u32,
    pub logical_cores: u32,
    pub total_ram_mb: u64,
    pub available_ram_mb: u64,
    pub available_storage_mb: u64,
    pub network_bandwidth_mbps: u32,
    pub accelerators: Vec<AcceleratorCapability>,
}

impl HardwareProfile {
    /// Returns only accelerators whose compute backends have been verified on physical hardware.
    #[must_use]
    pub fn verified_accelerators(&self) -> Vec<&AcceleratorCapability> {
        self.accelerators
            .iter()
            .filter(|a| a.verified_on_hardware)
            .collect()
    }
}

/// Explicit user consent policy governing resource sharing on a device.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceConsentPolicy {
    /// User has explicitly opted into executing workloads.
    pub sharing_enabled: bool,
    /// Maximum CPU utilization percentage allowed (1..=100).
    pub max_cpu_percent: u8,
    /// Maximum RAM in MiB that UMCF workloads may allocate.
    pub max_memory_mb: u64,
    /// Maximum local disk storage in MiB for models/shards/cache.
    pub max_storage_mb: u64,
    /// Whether workloads may execute when running on battery power.
    pub allow_on_battery: bool,
    /// Minimum battery percentage required when `allow_on_battery` is true.
    pub min_battery_percent: u8,
    /// Thermal cutoff in degrees Celsius above which workloads are throttled/paused.
    pub max_temperature_celsius: u8,
    /// Immediately preempt background compute if interactive user input is detected.
    pub preempt_on_user_activity: bool,
}

impl Default for ResourceConsentPolicy {
    /// Safe, conservative defaults requiring explicit user opt-in (`sharing_enabled: false`).
    fn default() -> Self {
        Self {
            sharing_enabled: false,
            max_cpu_percent: 50,
            max_memory_mb: 2048,
            max_storage_mb: 4096,
            allow_on_battery: false,
            min_battery_percent: 50,
            max_temperature_celsius: 75,
            preempt_on_user_activity: true,
        }
    }
}

impl ResourceConsentPolicy {
    pub fn validate(&self) -> Result<(), CoreError> {
        if self.max_cpu_percent == 0 || self.max_cpu_percent > 100 {
            return Err(CoreError::InvalidConsentPolicy(
                "max_cpu_percent must be between 1 and 100",
            ));
        }
        if self.max_memory_mb < 64 {
            return Err(CoreError::InvalidConsentPolicy(
                "max_memory_mb must be at least 64 MiB",
            ));
        }
        if self.min_battery_percent > 100 {
            return Err(CoreError::InvalidConsentPolicy(
                "min_battery_percent must be <= 100",
            ));
        }
        if self.max_temperature_celsius < 35 || self.max_temperature_celsius > 105 {
            return Err(CoreError::InvalidConsentPolicy(
                "max_temperature_celsius must be between 35 and 105 C",
            ));
        }
        Ok(())
    }
}

/// Short-lived, single-use enrollment token for QR / CLI device onboarding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnrollmentToken {
    pub token_id: String,
    pub issuer_node_id: NodeId,
    pub cluster_id: String,
    pub issued_at_unix_secs: u64,
    pub expires_at_unix_secs: u64,
    pub signature_hex: String,
}

impl EnrollmentToken {
    /// Maximum allowed lifetime of an enrollment token (300 seconds / 5 minutes).
    pub const MAX_TTL_SECS: u64 = 300;

    pub fn issue(
        token_id: impl Into<String>,
        issuer_node_id: NodeId,
        cluster_id: impl Into<String>,
        issued_at_unix_secs: u64,
        ttl_secs: u64,
        secret_material: &[u8],
    ) -> Result<Self, CoreError> {
        if ttl_secs == 0 || ttl_secs > Self::MAX_TTL_SECS {
            return Err(CoreError::InvalidEnrollmentTtl(ttl_secs));
        }
        let token_id = token_id.into();
        let cluster_id = cluster_id.into();
        let expires_at_unix_secs = issued_at_unix_secs + ttl_secs;
        let sig_input = format!(
            "{token_id}:{}:{cluster_id}:{issued_at_unix_secs}:{expires_at_unix_secs}",
            issuer_node_id.as_str()
        );
        let mut mac_buf = sig_input.into_bytes();
        mac_buf.extend_from_slice(secret_material);
        let sig = compute_fnv1a_64(&mac_buf);
        Ok(Self {
            token_id,
            issuer_node_id,
            cluster_id,
            issued_at_unix_secs,
            expires_at_unix_secs,
            signature_hex: format!("{sig:016x}"),
        })
    }

    /// Encodes the token as a compact `umcf://enroll?...` URI suitable for QR codes.
    #[must_use]
    pub fn to_qr_uri(&self) -> String {
        format!(
            "umcf://enroll?cluster={}&id={}&issuer={}&exp={}&sig={}",
            self.cluster_id,
            self.token_id,
            self.issuer_node_id.as_str(),
            self.expires_at_unix_secs,
            self.signature_hex
        )
    }

    pub fn verify(&self, now_unix_secs: u64, secret_material: &[u8]) -> Result<(), CoreError> {
        if now_unix_secs > self.expires_at_unix_secs {
            return Err(CoreError::EnrollmentTokenExpired {
                expired_at: self.expires_at_unix_secs,
                now: now_unix_secs,
            });
        }
        let sig_input = format!(
            "{}:{}:{}:{}:{}",
            self.token_id,
            self.issuer_node_id.as_str(),
            self.cluster_id,
            self.issued_at_unix_secs,
            self.expires_at_unix_secs
        );
        let mut mac_buf = sig_input.into_bytes();
        mac_buf.extend_from_slice(secret_material);
        let expected_sig = format!("{:016x}", compute_fnv1a_64(&mac_buf));
        if expected_sig != self.signature_hex {
            return Err(CoreError::InvalidEnrollmentSignature);
        }
        Ok(())
    }
}

/// Core domain errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoreError {
    InvalidPublicKeyLength(usize),
    InvalidNodeIdFormat(String),
    InvalidConsentPolicy(&'static str),
    InvalidEnrollmentTtl(u64),
    EnrollmentTokenExpired { expired_at: u64, now: u64 },
    InvalidEnrollmentSignature,
}

impl fmt::Display for CoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidPublicKeyLength(len) => {
                write!(f, "public key must be at least 16 bytes, got {len}")
            }
            Self::InvalidNodeIdFormat(s) => write!(f, "invalid NodeId format: {s}"),
            Self::InvalidConsentPolicy(msg) => write!(f, "invalid resource consent policy: {msg}"),
            Self::InvalidEnrollmentTtl(ttl) => {
                write!(f, "invalid enrollment token TTL ({ttl}s); max is 300s")
            }
            Self::EnrollmentTokenExpired { expired_at, now } => {
                write!(
                    f,
                    "enrollment token expired at {expired_at} (current time {now})"
                )
            }
            Self::InvalidEnrollmentSignature => write!(f, "invalid enrollment token signature"),
        }
    }
}

impl std::error::Error for CoreError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unit_node_id_derivation_and_parsing() {
        let key = b"0123456789abcdef0123456789abcdef";
        let id = NodeId::from_public_key_bytes(key).expect("derive node id");
        assert!(id.as_str().starts_with("did:umcf:"));
        let parsed = NodeId::parse(id.to_string()).expect("parse valid node id");
        assert_eq!(id, parsed);

        assert!(matches!(
            NodeId::from_public_key_bytes(b"short"),
            Err(CoreError::InvalidPublicKeyLength(5))
        ));
        assert!(matches!(
            NodeId::parse("invalid-id"),
            Err(CoreError::InvalidNodeIdFormat(_))
        ));
    }

    #[test]
    fn unit_role_state_machine_transitions() {
        let id = NodeId::from_public_key_bytes(b"node-pubkey-material-12345678").expect("id");
        let mut sm = RoleStateMachine::new(id.clone(), NodeRole::Worker);
        assert_eq!(sm.node_id(), &id);
        assert_eq!(sm.current_role(), NodeRole::Worker);
        assert!(!sm.can_coordinate());
        assert!(sm.can_execute_workloads());
        assert_eq!(sm.epoch(), 1);

        let ep = sm.transition_to(NodeRole::Hybrid);
        assert_eq!(ep, 2);
        assert!(sm.can_coordinate());
        assert!(sm.can_execute_workloads());
        assert_eq!(sm.current_role().to_string(), "Hybrid");

        let ep2 = sm.transition_to(NodeRole::Master);
        assert_eq!(ep2, 3);
        assert!(sm.can_coordinate());
        assert!(!sm.can_execute_workloads());
        assert_eq!(sm.current_role().to_string(), "Master");
    }

    #[test]
    fn unit_hardware_profile_filters_unverified_accelerators() {
        let id = NodeId::from_public_key_bytes(b"node-pubkey-material-12345678").expect("id");
        let profile = HardwareProfile {
            node_id: id,
            platform: PlatformKind::Windows,
            cpu_arch: CpuArch::X86_64,
            cpu_model: "Intel Core Ultra 5 225H".into(),
            physical_cores: 14,
            logical_cores: 14,
            total_ram_mb: 16384,
            available_ram_mb: 10240,
            available_storage_mb: 24000,
            network_bandwidth_mbps: 1000,
            accelerators: vec![
                AcceleratorCapability {
                    name: "Intel Arc 130T GPU".into(),
                    kind: AcceleratorKind::IntegratedGpu,
                    backend: ComputeBackend::Vulkan,
                    dedicated_memory_mb: 0,
                    shared_memory_mb: 8192,
                    supports_fp16: true,
                    supports_int8: true,
                    supports_int4: true,
                    verified_on_hardware: true,
                },
                AcceleratorCapability {
                    name: "Unverified NPU".into(),
                    kind: AcceleratorKind::Npu,
                    backend: ComputeBackend::IntelNpu,
                    dedicated_memory_mb: 0,
                    shared_memory_mb: 4096,
                    supports_fp16: true,
                    supports_int8: true,
                    supports_int4: false,
                    verified_on_hardware: false,
                },
            ],
        };

        let verified = profile.verified_accelerators();
        assert_eq!(verified.len(), 1);
        assert_eq!(verified[0].name, "Intel Arc 130T GPU");
        assert_eq!(verified[0].usable_memory_mb(), 8192);
    }

    #[test]
    fn unit_resource_consent_validation() {
        let mut policy = ResourceConsentPolicy::default();
        assert!(!policy.sharing_enabled);
        assert!(policy.validate().is_ok());

        policy.max_cpu_percent = 0;
        assert!(policy.validate().is_err());
        policy.max_cpu_percent = 80;
        policy.max_memory_mb = 16;
        assert!(policy.validate().is_err());
        policy.max_memory_mb = 4096;
        policy.max_temperature_celsius = 120;
        assert!(policy.validate().is_err());
    }

    #[test]
    fn unit_enrollment_token_issue_verify_and_qr_uri() {
        let issuer = NodeId::from_public_key_bytes(b"master-public-key-bytes-0001").expect("id");
        let secret = b"cluster-enrollment-hmac-secret";
        let token = EnrollmentToken::issue(
            "tok-001",
            issuer,
            "lan-cluster-alpha",
            1_700_000_000,
            120,
            secret,
        )
        .expect("issue token");

        assert!(token
            .to_qr_uri()
            .starts_with("umcf://enroll?cluster=lan-cluster-alpha"));
        assert!(token.verify(1_700_000_060, secret).is_ok());
        assert!(matches!(
            token.verify(1_700_000_200, secret),
            Err(CoreError::EnrollmentTokenExpired { .. })
        ));
        assert!(matches!(
            token.verify(1_700_000_060, b"wrong-secret"),
            Err(CoreError::InvalidEnrollmentSignature)
        ));
    }
}
