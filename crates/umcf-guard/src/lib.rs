//! `umcf-guard`: Local Resource Guard, Quota Enforcement & Consent Policy Lifecycle.
//! Enforces CPU shares, memory/VRAM limits, and cgroups v2/Job Object quotas for UMCF workers.
//! Strictly adheres to `#![forbid(unsafe_code)]` and zero placeholders.

#![forbid(unsafe_code)]

pub mod error;
pub mod lifecycle;
pub mod manager;
pub mod quota;
pub mod vram;

pub use error::{ErrorCode, GuardError};
pub use lifecycle::{
    AppState, GuardHysteresisConfig, GuardRegime, GuardTransitionResult, NodeGuardEvent,
    OsLifecycleSignal, SensorTelemetry, ThermalBatteryGuard, TrimMemoryLevel,
};
pub use manager::{QuotaManager, QuotaTelemetryReport, MAX_POLICY_UPDATE_LATENCY_MS};
pub use quota::{
    AllocatorGuard, CgroupV2Enforcer, JobObjectEnforcer, PosixRlimitEnforcer, QuotaConfig,
    QuotaEnforcer, RamReservationLease, WasmResourceLimiter,
};
pub use vram::{VramBudgetTracker, VramReservationLease};

