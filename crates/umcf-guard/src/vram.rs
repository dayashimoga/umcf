//! `umcf-guard::vram`: Atomic GPU/VRAM reservation accounting and RAII lease release.
//! Strictly enforces `#![forbid(unsafe_code)]` and zero placeholders.

#![forbid(unsafe_code)]

use crate::error::GuardError;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

/// Atomic VRAM reservation tracker preventing GPU/LLM out-of-memory overcommit.
#[derive(Debug)]
pub struct VramBudgetTracker {
    max_vram_bytes: AtomicU64,
    reserved_vram_bytes: AtomicU64,
}

impl VramBudgetTracker {
    /// Creates a new tracker with the specified consented maximum VRAM capacity.
    #[must_use]
    pub fn new(max_vram_bytes: u64) -> Arc<Self> {
        Arc::new(Self {
            max_vram_bytes: AtomicU64::new(max_vram_bytes),
            reserved_vram_bytes: AtomicU64::new(0),
        })
    }

    /// Atomically updates the maximum allowable VRAM allocation ceiling.
    pub fn update_limit(&self, new_max_bytes: u64) {
        self.max_vram_bytes.store(new_max_bytes, Ordering::Release);
    }

    /// Returns the active maximum VRAM ceiling.
    #[must_use]
    pub fn max_vram_bytes(&self) -> u64 {
        self.max_vram_bytes.load(Ordering::Acquire)
    }

    /// Returns currently reserved VRAM bytes.
    #[must_use]
    pub fn reserved_vram_bytes(&self) -> u64 {
        self.reserved_vram_bytes.load(Ordering::Acquire)
    }

    /// Returns currently available VRAM bytes before reaching the quota ceiling.
    #[must_use]
    pub fn available_vram_bytes(&self) -> u64 {
        let max = self.max_vram_bytes();
        let reserved = self.reserved_vram_bytes();
        max.saturating_sub(reserved)
    }

    /// Attempts to reserve the requested number of VRAM bytes for a task.
    ///
    /// # Errors
    /// Returns `GuardError::VramQuotaExceeded` if the allocation would exceed `max_vram_bytes`.
    pub fn reserve(
        self: &Arc<Self>,
        bytes: u64,
        task_id: &str,
    ) -> Result<VramReservationLease, GuardError> {
        let mut current_reserved = self.reserved_vram_bytes.load(Ordering::Acquire);

        loop {
            let ceiling = self.max_vram_bytes.load(Ordering::Acquire);
            let new_reserved = match current_reserved.checked_add(bytes) {
                Some(sum) if sum <= ceiling => sum,
                _ => {
                    let available = ceiling.saturating_sub(current_reserved);
                    return Err(GuardError::VramQuotaExceeded {
                        requested_bytes: bytes,
                        available_bytes: available,
                        ceiling_bytes: ceiling,
                    });
                }
            };

            match self.reserved_vram_bytes.compare_exchange_weak(
                current_reserved,
                new_reserved,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => {
                    let now_ms = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .map(|d| d.as_millis() as u64)
                        .unwrap_or(0);

                    return Ok(VramReservationLease {
                        tracker: Arc::clone(self),
                        bytes,
                        task_id: task_id.to_string(),
                        created_at_epoch_ms: now_ms,
                        is_active: true,
                    });
                }
                Err(actual) => current_reserved = actual,
            }
        }
    }

    /// Internal helper called by `VramReservationLease` upon drop or explicit release.
    fn release_bytes(&self, bytes: u64) {
        self.reserved_vram_bytes.fetch_sub(bytes, Ordering::AcqRel);
    }
}

/// RAII reservation lease ensuring 100% of allocated VRAM quota is immediately freed on drop.
#[derive(Debug)]
pub struct VramReservationLease {
    tracker: Arc<VramBudgetTracker>,
    bytes: u64,
    task_id: String,
    created_at_epoch_ms: u64,
    is_active: bool,
}

impl VramReservationLease {
    /// Returns the number of VRAM bytes reserved under this lease.
    #[must_use]
    pub const fn bytes(&self) -> u64 {
        self.bytes
    }

    /// Returns the task identifier bound to this lease.
    #[must_use]
    pub fn task_id(&self) -> &str {
        &self.task_id
    }

    /// Returns the timestamp when this lease was acquired.
    #[must_use]
    pub const fn created_at_epoch_ms(&self) -> u64 {
        self.created_at_epoch_ms
    }

    /// Explicitly releases the reserved VRAM quota back to the tracker.
    pub fn release(mut self) {
        if self.is_active {
            self.tracker.release_bytes(self.bytes);
            self.is_active = false;
        }
    }
}

impl Drop for VramReservationLease {
    fn drop(&mut self) {
        if self.is_active {
            self.tracker.release_bytes(self.bytes);
            self.is_active = false;
        }
    }
}
