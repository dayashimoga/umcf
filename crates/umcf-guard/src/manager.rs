//! `umcf-guard::manager`: High-level quota manager coordinating VRAM, RAM, and platform enforcers.
//! Strictly enforces `#![forbid(unsafe_code)]` and zero placeholders.

#![forbid(unsafe_code)]

use crate::error::GuardError;
use crate::quota::{
    AllocatorGuard, JobObjectEnforcer, QuotaConfig, QuotaEnforcer, RamReservationLease,
};
use crate::vram::{VramBudgetTracker, VramReservationLease};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, RwLock};
use umcf_enrollment::ResourceConsentPolicy;

/// Maximum allowable limit update latency (20 ms).
pub const MAX_POLICY_UPDATE_LATENCY_MS: u64 = 20;

/// Real-time quota and resource consumption metrics snapshot.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct QuotaTelemetryReport {
    pub max_cpu_percent: u8,
    pub max_ram_bytes: u64,
    pub reserved_ram_bytes: u64,
    pub available_ram_bytes: u64,
    pub max_vram_bytes: u64,
    pub reserved_vram_bytes: u64,
    pub available_vram_bytes: u64,
    pub policy_generation: u64,
}

/// Central resource guard orchestrator managing active consent limits across all subsystems.
#[derive(Debug)]
pub struct QuotaManager {
    active_policy: Arc<RwLock<ResourceConsentPolicy>>,
    vram_tracker: Arc<VramBudgetTracker>,
    allocator_guard: Arc<AllocatorGuard>,
    platform_enforcer: Arc<Mutex<Box<dyn QuotaEnforcer + Send + Sync>>>,
}

impl QuotaManager {
    /// Creates a new `QuotaManager` initialized with the given `ResourceConsentPolicy` and platform backend.
    pub fn new(
        initial_policy: ResourceConsentPolicy,
        platform_enforcer: Box<dyn QuotaEnforcer + Send + Sync>,
    ) -> Arc<Self> {
        let vram_tracker = VramBudgetTracker::new(initial_policy.max_vram_bytes.as_bytes());
        let allocator_guard = AllocatorGuard::new(initial_policy.max_ram_bytes.as_bytes());

        Arc::new(Self {
            active_policy: Arc::new(RwLock::new(initial_policy)),
            vram_tracker,
            allocator_guard,
            platform_enforcer: Arc::new(Mutex::new(platform_enforcer)),
        })
    }

    /// Creates a default `QuotaManager` utilizing the generic in-process/JobObject enforcer.
    #[must_use]
    pub fn with_default_enforcer(initial_policy: ResourceConsentPolicy) -> Arc<Self> {
        let quota_config = QuotaConfig::from_consent_policy(&initial_policy);
        let enforcer = Box::new(JobObjectEnforcer::new(quota_config));
        Self::new(initial_policy, enforcer)
    }

    /// Dynamically updates active resource limits whenever `ResourceConsentPolicy` changes.
    /// Invariant: Must apply all updates within `< 20 ms`.
    ///
    /// # Errors
    /// Returns `GuardError` if policy application or platform backend fails.
    pub async fn update_policy(
        &self,
        new_policy: ResourceConsentPolicy,
    ) -> Result<Duration, GuardError> {
        let t0 = Instant::now();

        let quota_config = QuotaConfig::from_consent_policy(&new_policy);

        // 1. Update VRAM tracker ceiling
        self.vram_tracker.update_limit(quota_config.max_vram_bytes);

        // 2. Update RAM allocator guard ceiling
        self.allocator_guard.update_limit(quota_config.max_ram_bytes);

        // 3. Update underlying OS/cgroup/JobObject platform enforcer
        {
            let mut enforcer = self.platform_enforcer.lock().await;
            enforcer.apply_limits(&quota_config)?;
        }

        // 4. Update stored policy
        {
            let mut policy_lock = self.active_policy.write().await;
            *policy_lock = new_policy;
        }

        let elapsed = t0.elapsed();
        Ok(elapsed)
    }

    /// Attempts to reserve VRAM bytes for a task, returning an RAII lease.
    ///
    /// # Errors
    /// Returns `GuardError::VramQuotaExceeded` if allocation exceeds policy limit.
    pub fn reserve_vram(
        &self,
        bytes: u64,
        task_id: &str,
    ) -> Result<VramReservationLease, GuardError> {
        self.vram_tracker.reserve(bytes, task_id)
    }

    /// Attempts to reserve RAM bytes for a task, returning an RAII lease.
    ///
    /// # Errors
    /// Returns `GuardError::MemoryQuotaExceeded` if allocation exceeds policy limit.
    pub fn reserve_ram(
        &self,
        bytes: u64,
        task_id: &str,
    ) -> Result<RamReservationLease, GuardError> {
        self.allocator_guard.reserve(bytes, task_id)
    }

    /// Assigns a running task to the platform quota group.
    ///
    /// # Errors
    /// Returns `GuardError` if platform assignment fails.
    pub async fn assign_task(&self, task_id: &str, pid: Option<u32>) -> Result<(), GuardError> {
        let mut enforcer = self.platform_enforcer.lock().await;
        enforcer.assign_task(task_id, pid)
    }

    /// Removes a task from the platform quota group.
    ///
    /// # Errors
    /// Returns `GuardError` if removal fails.
    pub async fn remove_task(&self, task_id: &str) -> Result<(), GuardError> {
        let mut enforcer = self.platform_enforcer.lock().await;
        enforcer.remove_task(task_id)
    }

    /// Generates a real-time telemetry snapshot of active resource limits and reservations.
    pub async fn telemetry(&self) -> QuotaTelemetryReport {
        let policy = self.active_policy.read().await;
        QuotaTelemetryReport {
            max_cpu_percent: policy.max_cpu_percent,
            max_ram_bytes: self.allocator_guard.max_ram_bytes(),
            reserved_ram_bytes: self.allocator_guard.reserved_ram_bytes(),
            available_ram_bytes: self.allocator_guard.available_ram_bytes(),
            max_vram_bytes: self.vram_tracker.max_vram_bytes(),
            reserved_vram_bytes: self.vram_tracker.reserved_vram_bytes(),
            available_vram_bytes: self.vram_tracker.available_vram_bytes(),
            policy_generation: policy.policy_generation,
        }
    }
}

