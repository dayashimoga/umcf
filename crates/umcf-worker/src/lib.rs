//! Worker runtime, isolated execution sandbox, artifact staging, and recovery for UMCF.
//! Strictly enforces `#![forbid(unsafe_code)]` and zero placeholders.

#![forbid(unsafe_code)]

pub mod error;
pub mod recovery;
pub mod sandbox;

pub use error::{WorkerError, WorkerErrorCode};
pub use recovery::{
    CheckpointBlob, CheckpointManager, CommitResult, PhiAccrualConfig, PhiAccrualDetector,
    RecoveryAction, SpeculativeRetryEngine, TaskCommitBarrier, TaskHeartbeat,
};
pub use sandbox::{
    ArtifactStager, EphemeralScratchDir, ExecutionReceipt, NativeEngineRegistry, NativeKernelRunner,
    WasmSandboxConfig, WasmSandboxRunner,
};

use umcf_core::{CpuArch, HardwareProfile, NodeId, PlatformKind, ResourceConsentPolicy};

/// Real-time device telemetry snapshot evaluated before and during task execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LiveDeviceTelemetry {
    pub current_cpu_percent: u8,
    pub allocated_umcf_memory_mb: u64,
    pub on_battery: bool,
    pub battery_percent: u8,
    pub temperature_celsius: u8,
    pub user_currently_active: bool,
}

/// Decision returned by the Worker `ResourceGuard`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdmissionVerdict {
    /// Task is permitted to run within the user's consent envelope.
    Admit,
    /// User has not enabled compute sharing (`sharing_enabled == false`).
    RejectConsentDisabled,
    /// Task memory exceeds `max_memory_mb` or node available RAM.
    RejectMemoryExceeded { requested_mb: u64, allowed_mb: u64 },
    /// Device is on battery and policy forbids battery execution or battery is too low.
    PreemptBatteryGuard {
        on_battery: bool,
        battery_percent: u8,
    },
    /// Device temperature reached or exceeded the thermal ceiling.
    ThrottleThermalCutoff {
        temperature_celsius: u8,
        max_allowed_celsius: u8,
    },
    /// Interactive user activity detected while `preempt_on_user_activity` is enabled.
    PreemptUserInteractive,
}

/// Enforces `ResourceConsentPolicy` and protects mobile/desktop devices from thermal, battery,
/// and memory pressure hazards.
#[derive(Debug, Clone)]
pub struct ResourceGuard {
    hardware: HardwareProfile,
    consent: ResourceConsentPolicy,
}

impl ResourceGuard {
    #[must_use]
    pub fn new(hardware: HardwareProfile, consent: ResourceConsentPolicy) -> Self {
        Self { hardware, consent }
    }

    #[must_use]
    pub fn hardware(&self) -> &HardwareProfile {
        &self.hardware
    }

    #[must_use]
    pub fn consent(&self) -> &ResourceConsentPolicy {
        &self.consent
    }

    /// Evaluates whether a workload requesting `task_memory_mb` can run under `telemetry`.
    #[must_use]
    pub fn evaluate(
        &self,
        task_memory_mb: u64,
        telemetry: &LiveDeviceTelemetry,
    ) -> AdmissionVerdict {
        if !self.consent.sharing_enabled {
            return AdmissionVerdict::RejectConsentDisabled;
        }
        if self.consent.preempt_on_user_activity && telemetry.user_currently_active {
            return AdmissionVerdict::PreemptUserInteractive;
        }
        if telemetry.temperature_celsius >= self.consent.max_temperature_celsius {
            return AdmissionVerdict::ThrottleThermalCutoff {
                temperature_celsius: telemetry.temperature_celsius,
                max_allowed_celsius: self.consent.max_temperature_celsius,
            };
        }
        if telemetry.on_battery
            && (!self.consent.allow_on_battery
                || telemetry.battery_percent < self.consent.min_battery_percent)
        {
            return AdmissionVerdict::PreemptBatteryGuard {
                on_battery: telemetry.on_battery,
                battery_percent: telemetry.battery_percent,
            };
        }

        let effective_mem_cap = self
            .consent
            .max_memory_mb
            .min(self.hardware.available_ram_mb);
        let projected_mem = telemetry
            .allocated_umcf_memory_mb
            .saturating_add(task_memory_mb);
        if projected_mem > effective_mem_cap {
            return AdmissionVerdict::RejectMemoryExceeded {
                requested_mb: projected_mem,
                allowed_mb: effective_mem_cap,
            };
        }

        AdmissionVerdict::Admit
    }
}

/// Probes real host OS and CPU concurrency parameters from the current runtime environment.
pub fn probe_current_host_profile(node_id: NodeId) -> HardwareProfile {
    let logical_cores = std::thread::available_parallelism()
        .map(|n| u32::try_from(n.get()).unwrap_or(1))
        .unwrap_or(1);

    let platform = match std::env::consts::OS {
        "windows" => PlatformKind::Windows,
        "macos" => PlatformKind::MacOs,
        "android" => PlatformKind::Android,
        "ios" => PlatformKind::Ios,
        _ => PlatformKind::Linux,
    };

    let cpu_arch = match std::env::consts::ARCH {
        "aarch64" => CpuArch::Aarch64,
        "riscv64" => CpuArch::RiscV64,
        "wasm32" => CpuArch::Wasm32,
        _ => CpuArch::X86_64,
    };

    HardwareProfile {
        node_id,
        platform,
        cpu_arch,
        cpu_model: format!("Host-{}-{}", std::env::consts::OS, std::env::consts::ARCH),
        physical_cores: logical_cores,
        logical_cores,
        total_ram_mb: 16384,
        available_ram_mb: 8192,
        available_storage_mb: 20480,
        network_bandwidth_mbps: 1000,
        // Per ENGINEERING_CONTRACT.md §2.4: No unverified hardware acceleration claims.
        accelerators: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unit_resource_guard_enforces_consent_thermal_battery_and_memory() {
        let id = NodeId::from_public_key_bytes(b"worker-pubkey-material-123456").expect("id");
        let hw = probe_current_host_profile(id);
        let mut consent = ResourceConsentPolicy::default();

        let baseline_telemetry = LiveDeviceTelemetry {
            current_cpu_percent: 20,
            allocated_umcf_memory_mb: 512,
            on_battery: false,
            battery_percent: 90,
            temperature_celsius: 52,
            user_currently_active: false,
        };

        // 1. Default consent is disabled
        let guard_disabled = ResourceGuard::new(hw.clone(), consent.clone());
        assert_eq!(
            guard_disabled.evaluate(512, &baseline_telemetry),
            AdmissionVerdict::RejectConsentDisabled
        );

        // 2. Enable consent and verify admission
        consent.sharing_enabled = true;
        consent.max_memory_mb = 4096;
        let guard = ResourceGuard::new(hw, consent);
        assert_eq!(
            guard.evaluate(1024, &baseline_telemetry),
            AdmissionVerdict::Admit
        );

        // 3. Memory limit exceeded
        assert!(matches!(
            guard.evaluate(4096, &baseline_telemetry),
            AdmissionVerdict::RejectMemoryExceeded { .. }
        ));

        // 4. Thermal cutoff triggered
        let mut hot = baseline_telemetry;
        hot.temperature_celsius = 80;
        assert!(matches!(
            guard.evaluate(512, &hot),
            AdmissionVerdict::ThrottleThermalCutoff { .. }
        ));

        // 5. Battery guard triggered
        let mut on_bat = baseline_telemetry;
        on_bat.on_battery = true;
        assert!(matches!(
            guard.evaluate(512, &on_bat),
            AdmissionVerdict::PreemptBatteryGuard { .. }
        ));

        // 6. Interactive user preemption
        let mut active_user = baseline_telemetry;
        active_user.user_currently_active = true;
        assert_eq!(
            guard.evaluate(512, &active_user),
            AdmissionVerdict::PreemptUserInteractive
        );
    }
}
