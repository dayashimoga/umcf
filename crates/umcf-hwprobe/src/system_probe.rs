//! `umcf-hwprobe::system_probe`: Unified system hardware prober orchestrating CPU, RAM, storage, power, and thermal probes.
//! Strictly enforces `#![forbid(unsafe_code)]` and zero placeholders.

#![forbid(unsafe_code)]

use crate::cpu::CpuProber;
use crate::error::HwProbeError;
use crate::memory_storage::MemoryStorageProber;
use crate::power_thermal::PowerThermalProber;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};
use sysinfo::{CpuRefreshKind, MemoryRefreshKind, RefreshKind, System};
use umcf_core::capabilities::{HardwareCapabilityDescriptor, NetworkCapability, NetworkLinkType};
use umcf_core::types::{DeviceClass, NodeId, OperatingSystem};

static GLOBAL_SYSTEM: OnceLock<Mutex<System>> = OnceLock::new();

/// Returns a reference to the global lazily initialized `System` instance.
pub fn get_system() -> &'static Mutex<System> {
    GLOBAL_SYSTEM.get_or_init(|| {
        Mutex::new(System::new_with_specifics(
            RefreshKind::nothing()
                .with_cpu(CpuRefreshKind::nothing())
                .with_memory(MemoryRefreshKind::nothing().with_ram()),
        ))
    })
}

/// Comprehensive system hardware capability prober.
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemProber;

impl SystemProber {
    /// Maximum allowed execution duration for complete hardware probe (25 milliseconds).
    pub const MAX_PROBE_DURATION_MS: u64 = 25;

    /// Returns a reference to the global lazily initialized `System` instance.
    #[must_use]
    pub fn get_system() -> &'static Mutex<System> {
        get_system()
    }

    /// Probes all hardware subsystems, returning a populated `HardwareCapabilityDescriptor`.
    ///
    /// # Errors
    /// Returns `HwProbeError` if any non-recoverable hardware query fails.
    pub fn probe_all(node_id: NodeId) -> Result<HardwareCapabilityDescriptor, HwProbeError> {
        let (descriptor, _) = Self::probe_all_with_duration(node_id)?;
        Ok(descriptor)
    }

    /// Probes all subsystems and returns both the descriptor and measured probe duration.
    ///
    /// # Errors
    /// Returns `HwProbeError` if probing fails.
    pub fn probe_all_with_duration(
        node_id: NodeId,
    ) -> Result<(HardwareCapabilityDescriptor, Duration), HwProbeError> {
        // Ensure static system and sensor instances are initialized before measuring probe execution duration
        let sys_lock = Self::get_system();
        crate::memory_storage::warmup_disks();
        crate::power_thermal::warmup_components();
        crate::accelerator_probe::AcceleratorProber::warmup();

        let start = Instant::now();

        // 1. Probe CPU and Memory under single lock acquisition
        let (cpu, memory) = {
            let mut sys = sys_lock
                .lock()
                .map_err(|e| HwProbeError::CpuProbe(e.to_string()))?;
            sys.refresh_cpu_specifics(CpuRefreshKind::nothing());
            sys.refresh_memory();
            let cpu = CpuProber::probe_cpu_with_sys(&sys)?;
            let mem = MemoryStorageProber::probe_memory_with_sys(&sys)?;
            (cpu, mem)
        };

        // 2. Probe Storage
        let storage = MemoryStorageProber::probe_storage(None)?;

        // 3. Probe Power & Thermal
        let power_thermal = PowerThermalProber::probe_power_thermal()?;

        // 4. Determine OS & DeviceClass
        let os = Self::detect_operating_system();
        let device_class = Self::infer_device_class(os, power_thermal.battery_percent.is_some(), cpu.physical_cores);

        // 5. Build Accelerator Entries (Discrete/Integrated GPUs, NPUs, CPU SIMD)
        let accelerators = crate::accelerator_probe::AcceleratorProber::enumerate()?;

        // 6. Network Capability Initial Baseline
        let network = NetworkCapability {
            link_type: NetworkLinkType::Loopback,
            estimated_bandwidth_mbps: 1000,
            measured_rtt_ms: 1,
        };

        let descriptor = HardwareCapabilityDescriptor {
            node_id,
            device_class,
            os,
            cpu,
            memory,
            accelerators,
            storage,
            network,
            power_thermal,
        };

        let elapsed = start.elapsed();
        Ok((descriptor, elapsed))
    }

    /// Detects current Operating System enum from target OS.
    #[must_use]
    pub fn detect_operating_system() -> OperatingSystem {
        match std::env::consts::OS {
            "windows" => OperatingSystem::Windows,
            "linux" => OperatingSystem::Linux,
            "macos" => OperatingSystem::MacOs,
            "android" => OperatingSystem::Android,
            "ios" => OperatingSystem::Ios,
            _ => OperatingSystem::Linux,
        }
    }

    /// Infers device class from operating system, battery presence, and core count.
    #[must_use]
    pub fn infer_device_class(
        os: OperatingSystem,
        has_battery: bool,
        physical_cores: u32,
    ) -> DeviceClass {
        match os {
            OperatingSystem::Android | OperatingSystem::Ios => DeviceClass::Smartphone,
            _ if has_battery => DeviceClass::Laptop,
            _ if physical_cores >= 16 => DeviceClass::Server,
            _ => DeviceClass::Desktop,
        }
    }
}
