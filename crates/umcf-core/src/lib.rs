//! Core domain models for Universal Mobile Compute Fabric (UMCF).
//! Implements node identity, dynamic Master/Worker/Hybrid roles, explicit heterogeneous hardware
//! capability profiles, units with checked arithmetic, and resource-sharing consent invariants.
//! Adheres strictly to `#![forbid(unsafe_code)]` and zero-placeholder invariants.

pub mod capabilities;
pub mod types;
pub mod units;

pub use capabilities::{
    AcceleratorCapability, AcceleratorKind, ComputeBackend, CpuCapability,
    HardwareCapabilityDescriptor, MemoryCapability, NetworkCapability, NetworkLinkType,
    PowerThermalState, ResourceRequirement, StorageCapability, ThermalState,
};
pub use types::{
    ClusterId, DeviceClass, JobId, NodeId, NodeRole, OperatingSystem, RoleStateMachine, TaskId,
};
pub use units::{ByteSize, MegabytesPerSec, MilliCores, TeraFlopsFp16};

use thiserror::Error;
use umcf_proto::compute_fnv1a_64;

/// Operating system and device form-factor classification (legacy profile compatibility).
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

/// Hardware capability profile of a single UMCF node.
#[derive(Debug, Clone, PartialEq)]
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

impl Eq for HardwareProfile {}

impl HardwareProfile {
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
    pub sharing_enabled: bool,
    pub max_cpu_percent: u8,
    pub max_memory_mb: u64,
    pub max_storage_mb: u64,
    pub allow_on_battery: bool,
    pub min_battery_percent: u8,
    pub max_temperature_celsius: u8,
    pub preempt_on_user_activity: bool,
}

impl Default for ResourceConsentPolicy {
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
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum CoreError {
    #[error("public key must be at least 16 bytes, got {0}")]
    InvalidPublicKeyLength(usize),

    #[error("invalid NodeId format: '{0}'")]
    InvalidNodeIdFormat(String),

    #[error("invalid ClusterId format: '{0}'")]
    InvalidClusterIdFormat(String),

    #[error("invalid resource consent policy: {0}")]
    InvalidConsentPolicy(&'static str),

    #[error("invalid enrollment token TTL ({0}s); max is 300s")]
    InvalidEnrollmentTtl(u64),

    #[error("enrollment token expired at {expired_at} (current time {now})")]
    EnrollmentTokenExpired { expired_at: u64, now: u64 },

    #[error("invalid enrollment token signature")]
    InvalidEnrollmentSignature,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;
    use umcf_proto::{MessageKind, PlaneKind, WireEnvelope};

    #[test]
    fn unit_node_id_derivation_and_parsing() {
        let key = b"ed25519-public-key-seed-node-01";
        let node_id = NodeId::from_public_key_bytes(key).expect("derivation");
        assert!(node_id.as_str().starts_with("did:umcf:"));

        let parsed = NodeId::parse(node_id.as_str()).expect("parse");
        assert_eq!(node_id, parsed);

        assert!(NodeId::from_public_key_bytes(b"short").is_err());
        assert!(NodeId::parse("invalid-did").is_err());
    }

    #[test]
    fn unit_role_state_machine_transitions() {
        let node_id = NodeId::parse("did:umcf:0123456789abcdef0123456789").expect("id");
        let mut sm = RoleStateMachine::new(node_id, NodeRole::Worker);
        assert_eq!(sm.current_role(), NodeRole::Worker);
        assert_eq!(sm.epoch(), 1);

        sm.transition_to(NodeRole::Hybrid);
        assert_eq!(sm.current_role(), NodeRole::Hybrid);
        assert_eq!(sm.epoch(), 2);
    }

    #[test]
    fn unit_units_checked_arithmetic_prevents_overflow() {
        let b1 = ByteSize::from_gib(10);
        let b2 = ByteSize::from_gib(5);
        assert_eq!(b1.checked_add(b2), Some(ByteSize::from_gib(15)));
        assert_eq!(b1.checked_sub(b2), Some(ByteSize::from_gib(5)));

        // Overflow prevention
        let max_bytes = ByteSize(u64::MAX);
        assert_eq!(max_bytes.checked_add(ByteSize(1)), None);
        assert_eq!(max_bytes.saturating_add(ByteSize(1)), ByteSize(u64::MAX));

        // MilliCores checked arithmetic
        let c1 = MilliCores::from_cores(4);
        let c2 = MilliCores::from_cores(2);
        assert_eq!(c1.checked_add(c2), Some(MilliCores::from_cores(6)));
        assert_eq!(c1.checked_sub(c2), Some(MilliCores::from_cores(2)));
        assert_eq!(MilliCores(u32::MAX).checked_add(MilliCores(1)), None);
    }

    #[allow(clippy::too_many_arguments)]
    fn sample_descriptor(
        name: &str,
        class: DeviceClass,
        os: OperatingSystem,
        cores: u32,
        ram_gib: u64,
        backend: Option<ComputeBackend>,
        vram_gib: u64,
        thermal: ThermalState,
        charging: bool,
    ) -> HardwareCapabilityDescriptor {
        let node_id = NodeId::parse(format!("did:umcf:{name}000000000000000000")).expect("id");
        let mut accelerators = Vec::new();
        if let Some(b) = backend {
            accelerators.push(AcceleratorCapability {
                name: format!("{name}-accelerator"),
                kind: AcceleratorKind::DiscreteGpu,
                backend: b,
                vram_bytes: ByteSize::from_gib(vram_gib),
                dedicated_memory_mb: vram_gib * 1024,
                shared_memory_mb: 0,
                compute_units: 32,
                fp32_tflops: 10.5,
                fp16_tflops: TeraFlopsFp16::from_tflops(21),
                int8_tops: 42,
                supports_fp16: true,
                supports_int8: true,
                supports_int4: true,
                verified_on_hardware: true,
            });
        }

        HardwareCapabilityDescriptor {
            node_id,
            device_class: class,
            os,
            cpu: CpuCapability {
                architecture: "x86_64".into(),
                physical_cores: cores,
                performance_cores: cores,
                efficiency_cores: 0,
                simd_avx2: true,
                simd_avx512: false,
                simd_neon: false,
                simd_sve: false,
                base_clock_mhz: 2400,
                boost_clock_mhz: 4800,
            },
            memory: MemoryCapability {
                total_ram: ByteSize::from_gib(ram_gib),
                available_ram: ByteSize::from_gib(ram_gib),
                is_unified_memory: os == OperatingSystem::MacOs || os == OperatingSystem::Ios,
                bandwidth_mbps: MegabytesPerSec::from_mbps(50_000),
            },
            accelerators,
            storage: StorageCapability {
                total_disk: ByteSize::from_gib(512),
                available_disk: ByteSize::from_gib(256),
                is_solid_state: true,
            },
            network: NetworkCapability {
                link_type: NetworkLinkType::Wifi,
                estimated_bandwidth_mbps: 500,
                measured_rtt_ms: 12,
            },
            power_thermal: PowerThermalState {
                battery_percent: if charging { Some(100) } else { Some(50) },
                is_charging: charging,
                thermal_state: thermal,
                temperature_milli_celsius: Some(45_000),
            },
        }
    }

    #[test]
    fn unit_hardware_capability_satisfies_across_25_profiles() {
        // Build 25 heterogeneous hardware profiles
        let profiles = [
            // 1-5: Smartphones & Tablets
            sample_descriptor(
                "phone-01",
                DeviceClass::Smartphone,
                OperatingSystem::Android,
                8,
                6,
                Some(ComputeBackend::Nnapi),
                2,
                ThermalState::Nominal,
                true,
            ),
            sample_descriptor(
                "phone-02",
                DeviceClass::Smartphone,
                OperatingSystem::Android,
                4,
                3,
                None,
                0,
                ThermalState::Fair,
                false,
            ),
            sample_descriptor(
                "phone-03",
                DeviceClass::Smartphone,
                OperatingSystem::Ios,
                6,
                6,
                Some(ComputeBackend::CoreMl),
                4,
                ThermalState::Nominal,
                true,
            ),
            sample_descriptor(
                "tablet-01",
                DeviceClass::Tablet,
                OperatingSystem::Android,
                8,
                8,
                Some(ComputeBackend::Vulkan),
                4,
                ThermalState::Nominal,
                true,
            ),
            sample_descriptor(
                "tablet-02",
                DeviceClass::Tablet,
                OperatingSystem::Ios,
                8,
                8,
                Some(ComputeBackend::Metal),
                8,
                ThermalState::Nominal,
                false,
            ),
            // 6-10: Laptops
            sample_descriptor(
                "laptop-01",
                DeviceClass::Laptop,
                OperatingSystem::Windows,
                8,
                16,
                Some(ComputeBackend::Dx12),
                6,
                ThermalState::Nominal,
                true,
            ),
            sample_descriptor(
                "laptop-02",
                DeviceClass::Laptop,
                OperatingSystem::Linux,
                16,
                32,
                Some(ComputeBackend::Vulkan),
                8,
                ThermalState::Fair,
                true,
            ),
            sample_descriptor(
                "laptop-03",
                DeviceClass::Laptop,
                OperatingSystem::MacOs,
                12,
                36,
                Some(ComputeBackend::Metal),
                36,
                ThermalState::Nominal,
                true,
            ),
            sample_descriptor(
                "laptop-04",
                DeviceClass::Laptop,
                OperatingSystem::Windows,
                4,
                8,
                None,
                0,
                ThermalState::Serious,
                false,
            ),
            sample_descriptor(
                "laptop-05",
                DeviceClass::Laptop,
                OperatingSystem::Linux,
                8,
                16,
                None,
                0,
                ThermalState::Nominal,
                true,
            ),
            // 11-15: Desktops & Workstations
            sample_descriptor(
                "desktop-01",
                DeviceClass::Desktop,
                OperatingSystem::Windows,
                16,
                32,
                Some(ComputeBackend::Cuda),
                16,
                ThermalState::Nominal,
                true,
            ),
            sample_descriptor(
                "desktop-02",
                DeviceClass::Desktop,
                OperatingSystem::Linux,
                24,
                64,
                Some(ComputeBackend::Cuda),
                24,
                ThermalState::Nominal,
                true,
            ),
            sample_descriptor(
                "desktop-03",
                DeviceClass::Desktop,
                OperatingSystem::Linux,
                16,
                32,
                Some(ComputeBackend::Rocm),
                16,
                ThermalState::Nominal,
                true,
            ),
            sample_descriptor(
                "workstation-01",
                DeviceClass::Workstation,
                OperatingSystem::Windows,
                32,
                128,
                Some(ComputeBackend::Cuda),
                48,
                ThermalState::Nominal,
                true,
            ),
            sample_descriptor(
                "workstation-02",
                DeviceClass::Workstation,
                OperatingSystem::Linux,
                64,
                256,
                Some(ComputeBackend::Cuda),
                96,
                ThermalState::Nominal,
                true,
            ),
            // 16-20: Servers
            sample_descriptor(
                "server-01",
                DeviceClass::Server,
                OperatingSystem::Linux,
                64,
                256,
                Some(ComputeBackend::Cuda),
                80,
                ThermalState::Nominal,
                true,
            ),
            sample_descriptor(
                "server-02",
                DeviceClass::Server,
                OperatingSystem::Linux,
                128,
                512,
                Some(ComputeBackend::Cuda),
                160,
                ThermalState::Nominal,
                true,
            ),
            sample_descriptor(
                "server-03",
                DeviceClass::Server,
                OperatingSystem::Linux,
                32,
                128,
                None,
                0,
                ThermalState::Nominal,
                true,
            ),
            sample_descriptor(
                "server-04",
                DeviceClass::Server,
                OperatingSystem::Linux,
                16,
                64,
                Some(ComputeBackend::OpenCl),
                16,
                ThermalState::Nominal,
                true,
            ),
            sample_descriptor(
                "server-05",
                DeviceClass::Server,
                OperatingSystem::Linux,
                8,
                32,
                None,
                0,
                ThermalState::Fair,
                true,
            ),
            // 21-25: Browser WASM & Degraded/Throttled Nodes
            sample_descriptor(
                "browser-01",
                DeviceClass::BrowserWasm,
                OperatingSystem::Web,
                4,
                4,
                Some(ComputeBackend::WebGpu),
                2,
                ThermalState::Nominal,
                true,
            ),
            sample_descriptor(
                "browser-02",
                DeviceClass::BrowserWasm,
                OperatingSystem::Web,
                2,
                2,
                None,
                0,
                ThermalState::Nominal,
                false,
            ),
            sample_descriptor(
                "phone-throttled",
                DeviceClass::Smartphone,
                OperatingSystem::Android,
                8,
                8,
                Some(ComputeBackend::Vulkan),
                4,
                ThermalState::Critical,
                false,
            ),
            sample_descriptor(
                "laptop-overheated",
                DeviceClass::Laptop,
                OperatingSystem::Windows,
                16,
                32,
                Some(ComputeBackend::Cuda),
                8,
                ThermalState::Critical,
                true,
            ),
            sample_descriptor(
                "desktop-cool",
                DeviceClass::Desktop,
                OperatingSystem::Linux,
                12,
                32,
                Some(ComputeBackend::Vulkan),
                12,
                ThermalState::Nominal,
                true,
            ),
        ];

        assert_eq!(profiles.len(), 25);

        // Test 1: High-end CUDA requirement (24 GB VRAM, AC required)
        let cuda_req = ResourceRequirement {
            min_cores: MilliCores::from_cores(16),
            min_ram: ByteSize::from_gib(32),
            min_vram: ByteSize::from_gib(24),
            required_backend: Some(ComputeBackend::Cuda),
            max_thermal_state: ThermalState::Fair,
            require_ac_power: true,
        };

        // Desktop-02, Workstation-01, Workstation-02, Server-01, Server-02 should satisfy
        assert!(profiles[11].satisfies(&cuda_req)); // desktop-02
        assert!(profiles[13].satisfies(&cuda_req)); // workstation-01
        assert!(profiles[14].satisfies(&cuda_req)); // workstation-02
        assert!(profiles[15].satisfies(&cuda_req)); // server-01

        // Phone-01, Browser-01, and Throttled profiles must NOT satisfy
        assert!(!profiles[0].satisfies(&cuda_req));
        assert!(!profiles[20].satisfies(&cuda_req));
        assert!(!profiles[22].satisfies(&cuda_req)); // critical thermal
        assert!(!profiles[23].satisfies(&cuda_req)); // critical thermal

        // Test 2: Low-end lightweight CPU task (2 cores, 2 GiB RAM, any backend, allows battery)
        let light_req = ResourceRequirement {
            min_cores: MilliCores::from_cores(2),
            min_ram: ByteSize::from_gib(2),
            min_vram: ByteSize::ZERO,
            required_backend: None,
            max_thermal_state: ThermalState::Fair,
            require_ac_power: false,
        };

        // Phone-01, Phone-02, Laptop-01, Desktop-01 should satisfy
        assert!(profiles[0].satisfies(&light_req));
        assert!(profiles[1].satisfies(&light_req));
        assert!(profiles[5].satisfies(&light_req));
        assert!(profiles[10].satisfies(&light_req));

        // Critical thermal profiles must still fail
        assert!(!profiles[22].satisfies(&light_req));
    }

    #[test]
    fn unit_serialization_bincode_and_json_parity() {
        let desc = sample_descriptor(
            "parity-test-node",
            DeviceClass::Workstation,
            OperatingSystem::Linux,
            32,
            128,
            Some(ComputeBackend::Cuda),
            24,
            ThermalState::Nominal,
            true,
        );

        // 1. JSON round-trip
        let json_str = serde_json::to_string_pretty(&desc).expect("serialize json");
        let from_json: HardwareCapabilityDescriptor =
            serde_json::from_str(&json_str).expect("deserialize json");
        assert_eq!(desc, from_json);

        // 2. Bincode binary round-trip
        let binary_bytes = bincode::serialize(&desc).expect("serialize bincode");
        let from_binary: HardwareCapabilityDescriptor =
            bincode::deserialize(&binary_bytes).expect("deserialize bincode");
        assert_eq!(desc, from_binary);
    }

    #[test]
    fn integration_embed_in_wire_envelope_and_export_schema() {
        let desc = sample_descriptor(
            "embed-test-node",
            DeviceClass::Desktop,
            OperatingSystem::Windows,
            16,
            32,
            Some(ComputeBackend::Dx12),
            12,
            ThermalState::Nominal,
            true,
        );

        // Serialize descriptor to binary payload
        let payload = bincode::serialize(&desc).expect("serialize bincode");

        // Embed inside WireEnvelope
        let env = WireEnvelope::new(
            PlaneKind::ControlPlane,
            MessageKind::CapabilityReport,
            1,
            desc.node_id.as_str(),
            payload,
        )
        .expect("envelope");

        let wire_bytes = env.to_bytes();
        let decoded_env = WireEnvelope::from_bytes(&wire_bytes).expect("decode wire envelope");

        let decoded_desc: HardwareCapabilityDescriptor =
            bincode::deserialize(&decoded_env.payload).expect("deserialize capability payload");
        assert_eq!(desc, decoded_desc);

        // Export JSON Schema to root schemas directory
        let schema = schemars::schema_for!(HardwareCapabilityDescriptor);
        let schema_json = serde_json::to_string_pretty(&schema).expect("schema json");

        let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        let root_schemas = manifest_dir
            .parent()
            .and_then(|p| p.parent())
            .map(|p| p.join("schemas"))
            .unwrap_or_else(|| std::path::PathBuf::from("schemas"));

        fs::create_dir_all(&root_schemas).expect("create schemas dir");
        let schema_path = root_schemas.join("hardware_capability_v1.schema.json");
        fs::write(&schema_path, schema_json).expect("write schema");
        assert!(schema_path.exists());
    }

    #[test]
    fn simulation_boundary_overflows_and_critical_rejection() {
        let mut desc = sample_descriptor(
            "boundary-node",
            DeviceClass::Smartphone,
            OperatingSystem::Android,
            8,
            8,
            None,
            0,
            ThermalState::Critical,
            false,
        );

        let req = ResourceRequirement {
            min_cores: MilliCores::from_cores(1),
            min_ram: ByteSize::from_gib(1),
            min_vram: ByteSize::ZERO,
            required_backend: None,
            max_thermal_state: ThermalState::Fair,
            require_ac_power: false,
        };

        // Critical thermal state must be rejected
        assert!(!desc.satisfies(&req));

        // 0 physical cores must be rejected
        desc.power_thermal.thermal_state = ThermalState::Nominal;
        desc.cpu.physical_cores = 0;
        assert!(!desc.satisfies(&req));

        // u64::MAX requirement must safely fail without panicking
        let huge_req = ResourceRequirement {
            min_cores: MilliCores(u32::MAX),
            min_ram: ByteSize(u64::MAX),
            min_vram: ByteSize(u64::MAX),
            required_backend: None,
            max_thermal_state: ThermalState::Critical,
            require_ac_power: false,
        };
        assert!(!desc.satisfies(&huge_req));
    }
}
