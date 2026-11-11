//! Token-bucket rate limiting and anti-replay window filtering for discovery packets.
//! Strictly enforces `#![forbid(unsafe_code)]` and zero-placeholder invariants.

use std::collections::HashMap;
use std::net::IpAddr;

use crate::error::NetError;

const DEFAULT_CAPACITY: f64 = 10.0;
const DEFAULT_REFILL_RATE_PER_SEC: f64 = 10.0;
const MAX_TRACKED_IPS: usize = 5_000;
const INACTIVE_IP_TIMEOUT_MS: u64 = 60_000;

#[derive(Debug, Clone)]
struct Bucket {
    tokens: f64,
    last_refill_ms: u64,
}

/// High-throughput token-bucket rate limiter enforcing max 10 beacons/sec per IP address.
#[derive(Debug, Clone)]
pub struct TokenBucketLimiter {
    capacity: f64,
    refill_rate_per_sec: f64,
    buckets: HashMap<IpAddr, Bucket>,
}

impl TokenBucketLimiter {
    /// Creates a new `TokenBucketLimiter` with default 10 tokens capacity and 10 tokens/sec refill rate.
    #[must_use]
    pub fn new() -> Self {
        Self::with_parameters(DEFAULT_CAPACITY, DEFAULT_REFILL_RATE_PER_SEC)
    }

    /// Creates a new `TokenBucketLimiter` with custom capacity and refill rate.
    #[must_use]
    pub fn with_parameters(capacity: f64, refill_rate_per_sec: f64) -> Self {
        Self {
            capacity,
            refill_rate_per_sec,
            buckets: HashMap::new(),
        }
    }

    /// Checks and consumes 1 token for an incoming IP address.
    ///
    /// # Errors
    /// Returns `NetError::RateLimited` if the IP address has exhausted its token quota.
    pub fn check_rate_limit(&mut self, ip: IpAddr, now_ms: u64) -> Result<(), NetError> {
        // Periodic cleanup of stale entries if bucket table grows too large
        if self.buckets.len() >= MAX_TRACKED_IPS {
            self.buckets
                .retain(|_, b| now_ms.saturating_sub(b.last_refill_ms) < INACTIVE_IP_TIMEOUT_MS);
        }

        let capacity = self.capacity;
        let refill_rate = self.refill_rate_per_sec;

        let bucket = self.buckets.entry(ip).or_insert_with(|| Bucket {
            tokens: capacity,
            last_refill_ms: now_ms,
        });

        // Refill tokens proportional to elapsed time
        let elapsed_ms = now_ms.saturating_sub(bucket.last_refill_ms);
        let elapsed_sec = elapsed_ms as f64 / 1000.0;
        bucket.tokens = (bucket.tokens + (elapsed_sec * refill_rate)).min(capacity);
        bucket.last_refill_ms = now_ms;

        if bucket.tokens >= 1.0 {
            bucket.tokens -= 1.0;
            Ok(())
        } else {
            Err(NetError::RateLimited {
                addr: ip.to_string(),
                count: (capacity - bucket.tokens) as usize,
            })
        }
    }

    /// Returns the number of actively tracked IP addresses.
    #[must_use]
    pub fn tracked_ip_count(&self) -> usize {
        self.buckets.len()
    }
}

impl Default for TokenBucketLimiter {
    fn default() -> Self {
        Self::new()
    }
}

/// Sliding replay window tracking peer sequence numbers to reject duplicate or old beacons.
#[derive(Debug, Clone)]
pub struct ReplayFilter {
    /// Maps sender DID -> (highest_seen_seq, bitmask_of_last_64_packets)
    windows: HashMap<String, (u64, u64)>,
}

impl ReplayFilter {
    /// Creates a new `ReplayFilter`.
    #[must_use]
    pub fn new() -> Self {
        Self {
            windows: HashMap::new(),
        }
    }

    /// Checks if a beacon with `sequence_number` from `sender_did` is fresh and valid.
    ///
    /// # Errors
    /// Returns `NetError::ReplayDetected` if duplicate or old packet.
    pub fn check_and_update(
        &mut self,
        sender_did: &str,
        seq: u64,
        timestamp_ms: u64,
    ) -> Result<(), NetError> {
        if self.windows.len() >= MAX_TRACKED_IPS {
            self.windows.clear(); // Reset filter when limit exceeded to maintain bounded memory
        }

        let entry = self.windows.entry(sender_did.to_string()).or_insert((0, 0));
        let (highest, bitmask) = entry;

        if seq > *highest {
            let diff = seq - *highest;
            if diff < 64 {
                *bitmask = (*bitmask << diff) | 1;
            } else {
                *bitmask = 1;
            }
            *highest = seq;
            Ok(())
        } else {
            let diff = *highest - seq;
            if diff >= 64 {
                // Too old, outside the 64-packet window
                return Err(NetError::ReplayDetected {
                    timestamp: timestamp_ms,
                    seq,
                });
            }

            let mask = 1_u64 << diff;
            if (*bitmask & mask) != 0 {
                // Already received!
                return Err(NetError::ReplayDetected {
                    timestamp: timestamp_ms,
                    seq,
                });
            }

            // Mark received in bitmask
            *bitmask |= mask;
            Ok(())
        }
    }
}

impl Default for ReplayFilter {
    fn default() -> Self {
        Self::new()
    }
}
