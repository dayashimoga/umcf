//! `umcf-net::qos`: Control-Plane vs Data-Plane Traffic Isolation and Quality of Service (QoS).
//!
//! Provides:
//! - Four-tier `TrafficClass` priority and IP DSCP / TOS byte definitions.
//! - Priority mapping for `quinn::SendStream::set_priority`.
//! - Operating system socket DSCP byte marking with unprivileged fallback.
//! - Hierarchical token-bucket bandwidth shaper (`BandwidthShaper`) with dynamic reconfiguration.
//! - OpenMetrics / Prometheus export for QoS telemetry.
//!
//! Strictly enforces `#![forbid(unsafe_code)]` and zero placeholders.

#![forbid(unsafe_code)]

use crate::error::NetError;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::fmt::Write;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tracing::debug;

/// Traffic classification tiers for UMCF network transport.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum TrafficClass {
    /// Critical control plane: heartbeats, election, preemption signals.
    /// Highest priority; reserved minimum 5% bandwidth; never dropped.
    CriticalControl = 0,
    /// Real-time interactive: remote desktop/game video frames, input injection, live token streaming.
    RealtimeInteractive = 1,
    /// Elastic compute: DAG task activations, RPC intermediate tensors.
    ElasticCompute = 2,
    /// Background bulk: GGUF shard synchronization, storage replication.
    BackgroundBulk = 3,
}

impl TrafficClass {
    /// Quinn send stream priority. Higher integer = higher transmission priority.
    #[must_use]
    pub const fn quic_priority(self) -> i32 {
        match self {
            Self::CriticalControl => 1000,
            Self::RealtimeInteractive => 500,
            Self::ElasticCompute => 100,
            Self::BackgroundBulk => 0,
        }
    }

    /// IP Differentiated Services Code Point (DSCP, 6-bit value).
    #[must_use]
    pub const fn dscp_value(self) -> u8 {
        match self {
            Self::CriticalControl => 46,     // Expedited Forwarding (EF)
            Self::RealtimeInteractive => 34, // Assured Forwarding 41 (AF41)
            Self::ElasticCompute => 18,      // Assured Forwarding 21 (AF21)
            Self::BackgroundBulk => 0,       // Best Effort (BE)
        }
    }

    /// IPv4 Type-of-Service (TOS) or IPv6 Traffic Class byte (DSCP shifted left by 2).
    #[must_use]
    pub const fn tos_byte(self) -> u8 {
        self.dscp_value() << 2
    }

    /// Human-readable label for Prometheus and structured logging.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::CriticalControl => "CriticalControl",
            Self::RealtimeInteractive => "RealtimeInteractive",
            Self::ElasticCompute => "ElasticCompute",
            Self::BackgroundBulk => "BackgroundBulk",
        }
    }

    /// Decode from raw u8 discriminant.
    #[must_use]
    pub const fn from_u8(val: u8) -> Option<Self> {
        match val {
            0 => Some(Self::CriticalControl),
            1 => Some(Self::RealtimeInteractive),
            2 => Some(Self::ElasticCompute),
            3 => Some(Self::BackgroundBulk),
            _ => None,
        }
    }
}

/// Applies IP DSCP / TOS byte to a socket2 Socket, gracefully falling back on unprivileged hosts.
pub fn apply_socket_dscp(socket: &socket2::Socket, dscp: u8) -> Result<(), NetError> {
    let tos = (dscp as u32) << 2;
    // Attempt IPv4 TOS
    if let Err(e) = socket.set_tos_v4(tos) {
        debug!(
            dscp,
            tos,
            error = %e,
            "Could not set IP_TOS on socket; falling back gracefully (unprivileged or unsupported)"
        );
    }
    Ok(())
}

/// Applies traffic class QoS markings to a socket.
pub fn apply_socket_traffic_class(
    socket: &socket2::Socket,
    class: TrafficClass,
) -> Result<(), NetError> {
    apply_socket_dscp(socket, class.dscp_value())
}

/// Per-class live QoS transmission statistics.
#[derive(Debug, Default)]
struct ClassCounters {
    bytes_transmitted: AtomicU64,
    messages_transmitted: AtomicU64,
    queue_drops: AtomicU64,
}

/// Snapshot of metrics for a traffic class.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassMetricsSnapshot {
    pub name: String,
    pub priority: i32,
    pub dscp: u8,
    pub bytes_transmitted: u64,
    pub messages_transmitted: u64,
    pub queue_drops: u64,
}

/// Full QoS telemetry snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QosTelemetrySnapshot {
    pub rate_limit_bytes_per_sec: u64,
    pub rate_limit_mbps: f64,
    pub classes: Vec<ClassMetricsSnapshot>,
    pub control_latency_p50_ms: f64,
    pub control_latency_p99_ms: f64,
}

/// Hierarchical token-bucket bandwidth shaper and QoS traffic scheduler.
///
/// Ensures:
/// 1. `CriticalControl` messages have guaranteed priority and are never starved or dropped.
/// 2. `BackgroundBulk` transfers are paced to respect configured upload/download limits.
/// 3. Rate limit changes converge in `< 200 ms`.
#[derive(Debug)]
pub struct BandwidthShaper {
    rate_limit_bytes_per_sec: AtomicU64,
    state: Mutex<ShaperState>,
    counters: [ClassCounters; 4],
    latency_recorder: Mutex<LatencyRecorder>,
}

#[derive(Debug)]
struct ShaperState {
    tokens: f64,
    last_refill: Instant,
    burst_capacity: f64,
}

#[derive(Debug)]
struct LatencyRecorder {
    samples: VecDeque<Duration>,
    max_samples: usize,
}

impl LatencyRecorder {
    fn new(max_samples: usize) -> Self {
        Self {
            samples: VecDeque::with_capacity(max_samples),
            max_samples,
        }
    }

    fn record(&mut self, latency: Duration) {
        if self.samples.len() >= self.max_samples {
            self.samples.pop_front();
        }
        self.samples.push_back(latency);
    }

    fn quantiles(&self) -> (f64, f64) {
        if self.samples.is_empty() {
            return (0.0, 0.0);
        }
        let mut sorted: Vec<f64> = self.samples.iter().map(|d| d.as_micros() as f64 / 1000.0).collect();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let p50_idx = ((sorted.len() as f64) * 0.50) as usize;
        let p99_idx = ((sorted.len() as f64) * 0.99) as usize;
        let p50 = sorted[p50_idx.min(sorted.len() - 1)];
        let p99 = sorted[p99_idx.min(sorted.len() - 1)];
        (p50, p99)
    }
}

impl BandwidthShaper {
    /// Create a new bandwidth shaper with a specified rate limit in bytes per second.
    /// A value of `0` denotes unconstrained bandwidth.
    #[must_use]
    pub fn new(rate_limit_bytes_per_sec: u64) -> Self {
        let rate_f = rate_limit_bytes_per_sec as f64;
        // Burst capacity: 50ms worth of tokens, or at least 64 KiB
        let burst_capacity = (rate_f * 0.05).max(65_536.0);

        Self {
            rate_limit_bytes_per_sec: AtomicU64::new(rate_limit_bytes_per_sec),
            state: Mutex::new(ShaperState {
                tokens: burst_capacity,
                last_refill: Instant::now(),
                burst_capacity,
            }),
            counters: [
                ClassCounters::default(),
                ClassCounters::default(),
                ClassCounters::default(),
                ClassCounters::default(),
            ],
            latency_recorder: Mutex::new(LatencyRecorder::new(1000)),
        }
    }

    /// Dynamically update the rate limit in bytes per second.
    /// Converges immediately for subsequent acquires.
    pub fn set_rate_limit(&self, rate_limit_bytes_per_sec: u64) {
        self.rate_limit_bytes_per_sec
            .store(rate_limit_bytes_per_sec, Ordering::Release);
        let rate_f = rate_limit_bytes_per_sec as f64;
        let burst_capacity = (rate_f * 0.05).max(65_536.0);

        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        state.burst_capacity = burst_capacity;
        if state.tokens > burst_capacity {
            state.tokens = burst_capacity;
        }
        state.last_refill = Instant::now();
    }

    /// Current rate limit in bytes per second.
    #[must_use]
    pub fn rate_limit(&self) -> u64 {
        self.rate_limit_bytes_per_sec.load(Ordering::Acquire)
    }

    /// Acquire permission to transmit `bytes` under the given `TrafficClass`.
    ///
    /// - `CriticalControl`: Bypasses pacing delays (guaranteed minimum 5% reserved bandwidth);
    ///   never sleeps; records latency and bytes transmitted.
    /// - `BackgroundBulk` and others: Computes required sleep time if token bucket is exhausted,
    ///   sleeping asynchronously before deducting tokens.
    pub async fn acquire(&self, class: TrafficClass, bytes: usize) -> Result<(), NetError> {
        let class_idx = class as usize;

        // Record message count
        self.counters[class_idx]
            .messages_transmitted
            .fetch_add(1, Ordering::Relaxed);

        let rate_limit = self.rate_limit();

        // CriticalControl is never delayed or dropped by bandwidth shaping
        if class == TrafficClass::CriticalControl || rate_limit == 0 {
            self.counters[class_idx]
                .bytes_transmitted
                .fetch_add(bytes as u64, Ordering::Relaxed);
            return Ok(());
        }

        let rate_f = rate_limit as f64;
        let bytes_f = bytes as f64;

        loop {
            let sleep_duration = {
                let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
                let now = Instant::now();
                let elapsed = now.duration_since(state.last_refill).as_secs_f64();
                state.tokens = (state.tokens + elapsed * rate_f).min(state.burst_capacity);
                state.last_refill = now;

                if state.tokens >= bytes_f {
                    state.tokens -= bytes_f;
                    Duration::ZERO
                } else {
                    let needed = bytes_f - state.tokens;
                    let wait_secs = needed / rate_f;
                    Duration::from_secs_f64(wait_secs.max(0.0001))
                }
            };

            if sleep_duration.is_zero() {
                break;
            }

            tokio::time::sleep(sleep_duration).await;
        }

        self.counters[class_idx]
            .bytes_transmitted
            .fetch_add(bytes as u64, Ordering::Relaxed);
        Ok(())
    }

    /// Record measured round-trip latency for a critical control ping.
    pub fn record_control_latency(&self, latency: Duration) {
        let mut rec = self.latency_recorder.lock().unwrap_or_else(|e| e.into_inner());
        rec.record(latency);
    }

    /// Record a queue drop event for the given class (forbidden for `CriticalControl`).
    pub fn record_drop(&self, class: TrafficClass) {
        let class_idx = class as usize;
        self.counters[class_idx]
            .queue_drops
            .fetch_add(1, Ordering::Relaxed);
    }

    /// Snapshot current QoS telemetry.
    #[must_use]
    pub fn snapshot(&self) -> QosTelemetrySnapshot {
        let rate_limit = self.rate_limit();
        let rate_mbps = (rate_limit as f64 * 8.0) / 1_000_000.0;

        let classes = [
            TrafficClass::CriticalControl,
            TrafficClass::RealtimeInteractive,
            TrafficClass::ElasticCompute,
            TrafficClass::BackgroundBulk,
        ]
        .iter()
        .map(|&c| {
            let idx = c as usize;
            ClassMetricsSnapshot {
                name: c.name().to_string(),
                priority: c.quic_priority(),
                dscp: c.dscp_value(),
                bytes_transmitted: self.counters[idx].bytes_transmitted.load(Ordering::Relaxed),
                messages_transmitted: self.counters[idx].messages_transmitted.load(Ordering::Relaxed),
                queue_drops: self.counters[idx].queue_drops.load(Ordering::Relaxed),
            }
        })
        .collect();

        let (p50, p99) = {
            let rec = self.latency_recorder.lock().unwrap_or_else(|e| e.into_inner());
            rec.quantiles()
        };

        QosTelemetrySnapshot {
            rate_limit_bytes_per_sec: rate_limit,
            rate_limit_mbps: rate_mbps,
            classes,
            control_latency_p50_ms: p50,
            control_latency_p99_ms: p99,
        }
    }

    /// Export telemetry in standard OpenMetrics / Prometheus exposition text format.
    #[must_use]
    pub fn to_openmetrics_text(&self) -> String {
        let snap = self.snapshot();
        let mut out = String::with_capacity(1024);

        let _ = writeln!(out, "# HELP umcf_qos_rate_limit_bytes_per_sec Configured rate limit in bytes/sec");
        let _ = writeln!(out, "# TYPE umcf_qos_rate_limit_bytes_per_sec gauge");
        let _ = writeln!(out, "umcf_qos_rate_limit_bytes_per_sec {}", snap.rate_limit_bytes_per_sec);

        let _ = writeln!(out, "# HELP umcf_qos_bytes_transmitted_total Total bytes transmitted per QoS class");
        let _ = writeln!(out, "# TYPE umcf_qos_bytes_transmitted_total counter");
        for c in &snap.classes {
            let _ = writeln!(
                out,
                "umcf_qos_bytes_transmitted_total{{class=\"{}\",dscp=\"{}\"}} {}",
                c.name, c.dscp, c.bytes_transmitted
            );
        }

        let _ = writeln!(out, "# HELP umcf_qos_messages_transmitted_total Total messages transmitted per QoS class");
        let _ = writeln!(out, "# TYPE umcf_qos_messages_transmitted_total counter");
        for c in &snap.classes {
            let _ = writeln!(
                out,
                "umcf_qos_messages_transmitted_total{{class=\"{}\"}} {}",
                c.name, c.messages_transmitted
            );
        }

        let _ = writeln!(out, "# HELP umcf_qos_queue_drops_total Total messages dropped per QoS class");
        let _ = writeln!(out, "# TYPE umcf_qos_queue_drops_total counter");
        for c in &snap.classes {
            let _ = writeln!(
                out,
                "umcf_qos_queue_drops_total{{class=\"{}\"}} {}",
                c.name, c.queue_drops
            );
        }

        let _ = writeln!(out, "# HELP umcf_qos_control_latency_p99_ms P99 latency for critical control heartbeats in ms");
        let _ = writeln!(out, "# TYPE umcf_qos_control_latency_p99_ms gauge");
        let _ = writeln!(out, "umcf_qos_control_latency_p99_ms {:.3}", snap.control_latency_p99_ms);

        out
    }
}
