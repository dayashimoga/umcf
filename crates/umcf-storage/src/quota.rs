//! Byte-Exact Atomic Quota Enforcer & Host OS Floor Protection (UMCF S7.P1.SP1).
//!
//! Enforces lock-free compare-and-swap reservation permits, host OS free disk floor
//! protection (15% safety floor), and atomic accounting across concurrent writes.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use crate::error::StorageError;

/// RAII permit for in-flight storage writes.
/// Automatically releases uncommitted reserved bytes upon drop (e.g. on I/O error or hash mismatch).
#[derive(Debug)]
pub struct QuotaReservationPermit {
    enforcer: Arc<StrictQuotaEnforcer>,
    pub bytes_reserved: u64,
    pub committed: bool,
}

impl QuotaReservationPermit {
    /// Commits the in-flight reservation into permanent used storage.
    pub fn commit(&mut self, actual_bytes: u64) {
        if self.committed {
            return;
        }
        self.committed = true;

        // Atomically transfer from in-flight reservation to permanent used storage
        self.enforcer
            .reserved_in_flight_bytes
            .fetch_sub(self.bytes_reserved, Ordering::SeqCst);
        self.enforcer
            .used_bytes
            .fetch_add(actual_bytes, Ordering::SeqCst);
    }
}

impl Drop for QuotaReservationPermit {
    fn drop(&mut self) {
        if !self.committed {
            self.enforcer
                .reserved_in_flight_bytes
                .fetch_sub(self.bytes_reserved, Ordering::SeqCst);
        }
    }
}

/// Byte-Exact Atomic Storage Quota Enforcer.
#[derive(Debug)]
pub struct StrictQuotaEnforcer {
    pub used_bytes: AtomicU64,
    pub reserved_in_flight_bytes: AtomicU64,
    pub configured_max_bytes: AtomicU64,
    pub host_free_floor_bytes: u64,
}

impl StrictQuotaEnforcer {
    #[must_use]
    pub fn new(configured_max_bytes: u64, host_free_floor_bytes: u64) -> Arc<Self> {
        Arc::new(Self {
            used_bytes: AtomicU64::new(0),
            reserved_in_flight_bytes: AtomicU64::new(0),
            configured_max_bytes: AtomicU64::new(configured_max_bytes),
            host_free_floor_bytes,
        })
    }

    /// Calculates the effective storage ceiling considering host OS free space floor.
    #[must_use]
    pub fn calculate_effective_limit(&self, host_free_bytes: u64) -> u64 {
        let configured = self.configured_max_bytes.load(Ordering::Relaxed);
        let used = self.used_bytes.load(Ordering::Relaxed);

        let allowable_from_host = if host_free_bytes > self.host_free_floor_bytes {
            used.saturating_add(host_free_bytes - self.host_free_floor_bytes)
        } else {
            used
        };

        configured.min(allowable_from_host)
    }

    /// Attempts to reserve in-flight storage bytes via a lock-free compare-and-swap loop.
    pub fn try_reserve(
        self: &Arc<Self>,
        bytes: u64,
        host_free_bytes: u64,
    ) -> Result<QuotaReservationPermit, StorageError> {
        if bytes == 0 {
            return Ok(QuotaReservationPermit {
                enforcer: Arc::clone(self),
                bytes_reserved: 0,
                committed: false,
            });
        }

        // 1. Host OS floor protection check
        if host_free_bytes <= self.host_free_floor_bytes {
            return Err(StorageError::HostSpaceExhausted {
                host_free_bytes,
                safety_floor_bytes: self.host_free_floor_bytes,
            });
        }

        let effective_limit = self.calculate_effective_limit(host_free_bytes);

        // 2. Lock-free CAS loop on reserved_in_flight_bytes
        let mut current_reserved = self.reserved_in_flight_bytes.load(Ordering::SeqCst);
        loop {
            let current_used = self.used_bytes.load(Ordering::SeqCst);
            let total_allocated = current_used.saturating_add(current_reserved);
            let new_total = total_allocated.saturating_add(bytes);

            if new_total > effective_limit {
                return Err(StorageError::QuotaExceeded {
                    requested_bytes: bytes,
                    available_bytes: effective_limit.saturating_sub(total_allocated),
                    effective_limit_bytes: effective_limit,
                });
            }

            match self.reserved_in_flight_bytes.compare_exchange_weak(
                current_reserved,
                current_reserved + bytes,
                Ordering::SeqCst,
                Ordering::SeqCst,
            ) {
                Ok(_) => {
                    return Ok(QuotaReservationPermit {
                        enforcer: Arc::clone(self),
                        bytes_reserved: bytes,
                        committed: false,
                    });
                }
                Err(actual) => {
                    current_reserved = actual;
                }
            }
        }
    }

    /// Releases unreserved or purged storage bytes from permanent used storage.
    pub fn release_used_bytes(&self, bytes: u64) {
        let mut current = self.used_bytes.load(Ordering::SeqCst);
        loop {
            let next = current.saturating_sub(bytes);
            match self.used_bytes.compare_exchange_weak(
                current,
                next,
                Ordering::SeqCst,
                Ordering::SeqCst,
            ) {
                Ok(_) => break,
                Err(actual) => current = actual,
            }
        }
    }

    /// Sets a new storage ceiling.
    pub fn set_configured_max_bytes(&self, new_max_bytes: u64) {
        self.configured_max_bytes.store(new_max_bytes, Ordering::SeqCst);
    }

    /// Returns the currently committed used bytes on disk.
    #[must_use]
    pub fn used_bytes(&self) -> u64 {
        self.used_bytes.load(Ordering::SeqCst)
    }

    /// Alias for release_used_bytes when sweeping chunks during garbage collection.
    pub fn release_swept_bytes(&self, bytes: u64) {
        self.release_used_bytes(bytes);
    }

    /// Returns the sum of used and currently in-flight reserved bytes.
    #[must_use]
    pub fn total_allocated_bytes(&self) -> u64 {
        self.used_bytes
            .load(Ordering::SeqCst)
            .saturating_add(self.reserved_in_flight_bytes.load(Ordering::SeqCst))
    }
}

