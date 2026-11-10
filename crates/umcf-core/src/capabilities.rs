//! Explicit hardware, accelerator, power, and thermal capability descriptors for UMCF.
//! Adheres strictly to `#![forbid(unsafe_code)]` and zero-placeholder invariants.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt;

use crate::types::{DeviceClass, NodeId, OperatingSystem};
use crate::units::{ByteSize, MegabytesPerSec, MilliCores, TeraFlopsFp16};

/// Hardware execution and accelerator backend APIs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema, Default)]
pub enum ComputeBackend {
    #[default]
    CpuSimd,
    Vulkan,
    Metal,
    Dx12,
    Cuda,
    Rocm,
    OpenCl,
    Nnapi,
    CoreMl,
    WebGpu,
}

impl fmt::Display for ComputeBackend {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CpuSimd => write!(f, "CPU SIMD"),
            Self::Vulkan => write!(f, "Vulkan"),
            Self::Metal => write!(f, "Metal"),
            Self::Dx12 => write!(f, "DirectX 12"),
            Self::Cuda => write!(f, "CUDA"),
            Self::Rocm => write!(f, "ROCm"),
            Self::OpenCl => write!(f, "OpenCL"),
            Self::Nnapi => write!(f, "Android NNAPI"),
            Self::CoreMl => write!(f, "Apple CoreML"),
            Self::WebGpu => write!(f, "WebGPU"),
        }
    }
}

/// Hardware accelerator category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema, Default)]
pub enum AcceleratorKind {
    #[default]
    DiscreteGpu,
    IntegratedGpu,
    MobileGpu,
    Npu,
    Cpu,
}

impl fmt::Display for AcceleratorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DiscreteGpu => write!(f, "Discrete GPU"),
            Self::IntegratedGpu => write!(f, "Integrated GPU"),
            Self::MobileGpu => write!(f, "Mobile GPU"),
            Self::Npu => write!(f, "NPU"),
            Self::Cpu => write!(f, "CPU"),
        }
    }
}

/// OS thermal throttle status.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Serialize,
    Deserialize,
    JsonSchema,
    Default,
)]
pub enum ThermalState {
    #[default]
    Nominal = 0,
    Fair = 1,
    Serious = 2,
    Critical = 3,
}

impl fmt::Display for ThermalState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Nominal => write!(f, "Nominal"),
            Self::Fair => write!(f, "Fair"),
            Self::Serious => write!(f, "Serious"),
            Self::Critical => write!(f, "Critical"),
        }
    }
}

/// Network link medium.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema, Default)]
pub enum NetworkLinkType {
    #[default]
    Unknown,
    Loopback,
    Ethernet,
    Wifi,
    Cellular5G,
    Cellular4G,
}

impl fmt::Display for NetworkLinkType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unknown => write!(f, "Unknown"),
            Self::Loopback => write!(f, "Loopback"),
            Self::Ethernet => write!(f, "Ethernet"),
            Self::Wifi => write!(f, "Wi-Fi"),
            Self::Cellular5G => write!(f, "Cellular (5G)"),
            Self::Cellular4G => write!(f, "Cellular (4G)"),
        }
    }
}

/// CPU hardware specifications and SIMD feature set.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
pub struct CpuCapability {
    pub architecture: String,
    pub physical_cores: u32,
    pub performance_cores: u32,
    pub efficiency_cores: u32,
    pub simd_avx2: bool,
    pub simd_avx512: bool,
    pub simd_neon: bool,
    pub simd_sve: bool,
    pub base_clock_mhz: u32,
    pub boost_clock_mhz: u32,
}

/// System RAM specifications and bandwidth.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
pub struct MemoryCapability {
    pub total_ram: ByteSize,
    pub available_ram: ByteSize,
    pub is_unified_memory: bool,
    pub bandwidth_mbps: MegabytesPerSec,
}

/// Discrete or integrated GPU/NPU accelerator capabilities.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct AcceleratorCapability {
    pub name: String,
    pub kind: AcceleratorKind,
    pub backend: ComputeBackend,
    pub vram_bytes: ByteSize,
    pub dedicated_memory_mb: u64,
    pub shared_memory_mb: u64,
    pub compute_units: u32,
    pub fp32_tflops: f32,
    pub fp16_tflops: TeraFlopsFp16,
    pub int8_tops: u32,
    pub supports_fp16: bool,
    pub supports_int8: bool,
    pub supports_int4: bool,
    pub verified_on_hardware: bool,
}

impl Eq for AcceleratorCapability {}

impl AcceleratorCapability {
    #[must_use]
    pub const fn usable_memory_mb(&self) -> u64 {
        if self.dedicated_memory_mb > 0 {
            self.dedicated_memory_mb
        } else if self.shared_memory_mb > 0 {
            self.shared_memory_mb
        } else {
            self.vram_bytes.0 / (1024 * 1024)
        }
    }
}

/// Local persistent storage metrics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
pub struct StorageCapability {
    pub total_disk: ByteSize,
    pub available_disk: ByteSize,
    pub is_solid_state: bool,
}

/// Current network connectivity state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
pub struct NetworkCapability {
    pub link_type: NetworkLinkType,
    pub estimated_bandwidth_mbps: u32,
    pub measured_rtt_ms: u32,
}

/// Real-time battery and thermal status of the device.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
pub struct PowerThermalState {
    pub battery_percent: Option<u8>,
    pub is_charging: bool,
    pub thermal_state: ThermalState,
    pub temperature_milli_celsius: Option<i32>,
}

/// Complete multi-dimensional hardware capability descriptor advertised by a node.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct HardwareCapabilityDescriptor {
    pub node_id: NodeId,
    pub device_class: DeviceClass,
    pub os: OperatingSystem,
    pub cpu: CpuCapability,
    pub memory: MemoryCapability,
    pub accelerators: Vec<AcceleratorCapability>,
    pub storage: StorageCapability,
    pub network: NetworkCapability,
    pub power_thermal: PowerThermalState,
}

impl Eq for HardwareCapabilityDescriptor {}

/// Task or DAG stage hardware requirement profile.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ResourceRequirement {
    pub min_cores: MilliCores,
    pub min_ram: ByteSize,
    pub min_vram: ByteSize,
    pub required_backend: Option<ComputeBackend>,
    pub max_thermal_state: ThermalState,
    pub require_ac_power: bool,
}

impl Default for ResourceRequirement {
    fn default() -> Self {
        Self {
            min_cores: MilliCores::from_cores(1),
            min_ram: ByteSize::from_mib(512),
            min_vram: ByteSize::ZERO,
            required_backend: None,
            max_thermal_state: ThermalState::Fair,
            require_ac_power: false,
        }
    }
}

impl HardwareCapabilityDescriptor {
    /// Evaluates whether this node's current hardware state satisfies all dimensions of a requirement.
    #[must_use]
    pub fn satisfies(&self, req: &ResourceRequirement) -> bool {
        // 1. CPU Core count
        let total_milli_cores = self.cpu.physical_cores.saturating_mul(1000);
        if total_milli_cores < req.min_cores.as_millis() {
            return false;
        }

        // 2. Available RAM
        if self.memory.available_ram.as_bytes() < req.min_ram.as_bytes() {
            return false;
        }

        // 3. Thermal state ceiling
        if self.power_thermal.thermal_state > req.max_thermal_state {
            return false;
        }

        // 4. AC power constraint
        if req.require_ac_power {
            let on_ac =
                self.power_thermal.is_charging || self.power_thermal.battery_percent.is_none();
            if !on_ac {
                return false;
            }
        }

        // 5. Accelerator / VRAM constraint
        if req.min_vram.as_bytes() > 0 || req.required_backend.is_some() {
            let mut satisfied = false;
            for acc in &self.accelerators {
                let backend_match = match req.required_backend {
                    Some(b) => acc.backend == b,
                    None => true,
                };
                let vram_match = if self.memory.is_unified_memory {
                    self.memory.available_ram.as_bytes() >= req.min_vram.as_bytes()
                } else {
                    acc.vram_bytes.as_bytes() >= req.min_vram.as_bytes()
                        || (acc.dedicated_memory_mb.saturating_mul(1024 * 1024)
                            >= req.min_vram.as_bytes())
                };

                if backend_match && vram_match {
                    satisfied = true;
                    break;
                }
            }
            if !satisfied {
                return false;
            }
        }

        true
    }
}
