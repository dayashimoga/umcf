//! Lock-free atomic metrics and OpenMetrics text format exporter for UMCF.
//! Adheres strictly to `#![forbid(unsafe_code)]` and zero-placeholder invariants.

use std::fmt::Write as _;
use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};
use std::sync::Arc;

/// Lock-free monotonic 64-bit integer counter.
#[derive(Debug, Default)]
pub struct Counter {
    val: AtomicU64,
}

impl Counter {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            val: AtomicU64::new(0),
        }
    }

    pub fn inc(&self) {
        self.val.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_by(&self, amount: u64) {
        self.val.fetch_add(amount, Ordering::Relaxed);
    }

    #[must_use]
    pub fn get(&self) -> u64 {
        self.val.load(Ordering::Relaxed)
    }
}

/// Lock-free 64-bit gauge supporting positive and negative adjustments.
#[derive(Debug, Default)]
pub struct Gauge {
    val: AtomicI64,
}

impl Gauge {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            val: AtomicI64::new(0),
        }
    }

    pub fn set(&self, value: i64) {
        self.val.store(value, Ordering::Relaxed);
    }

    pub fn inc(&self) {
        self.val.fetch_add(1, Ordering::Relaxed);
    }

    pub fn dec(&self) {
        self.val.fetch_sub(1, Ordering::Relaxed);
    }

    #[must_use]
    pub fn get(&self) -> i64 {
        self.val.load(Ordering::Relaxed)
    }
}

/// Fixed-bucket latency histogram with lock-free atomic bucket counts.
#[derive(Debug)]
pub struct LatencyHistogram {
    buckets: Vec<f64>,
    counts: Vec<AtomicU64>,
    sum_micros: AtomicU64,
    total_count: AtomicU64,
}

impl LatencyHistogram {
    #[must_use]
    pub fn new(buckets: Vec<f64>) -> Self {
        let len = buckets.len();
        let mut counts = Vec::with_capacity(len);
        for _ in 0..len {
            counts.push(AtomicU64::new(0));
        }
        Self {
            buckets,
            counts,
            sum_micros: AtomicU64::new(0),
            total_count: AtomicU64::new(0),
        }
    }

    /// Observes a duration in seconds (e.g. 0.0025 s = 2.5 ms).
    pub fn observe(&self, seconds: f64) {
        let micros = (seconds * 1_000_000.0) as u64;
        self.sum_micros.fetch_add(micros, Ordering::Relaxed);
        self.total_count.fetch_add(1, Ordering::Relaxed);

        for (i, &upper_bound) in self.buckets.iter().enumerate() {
            if seconds <= upper_bound {
                self.counts[i].fetch_add(1, Ordering::Relaxed);
                break;
            }
        }
    }

    #[must_use]
    pub fn count(&self) -> u64 {
        self.total_count.load(Ordering::Relaxed)
    }

    #[must_use]
    pub fn sum_seconds(&self) -> f64 {
        self.sum_micros.load(Ordering::Relaxed) as f64 / 1_000_000.0
    }
}

/// Metric descriptor and exporter registry.
#[derive(Debug, Default)]
pub struct MetricsRegistry {
    counters: Vec<(&'static str, &'static str, Arc<Counter>)>,
    gauges: Vec<(&'static str, &'static str, Arc<Gauge>)>,
    histograms: Vec<(&'static str, &'static str, Arc<LatencyHistogram>)>,
}

impl MetricsRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_counter(
        &mut self,
        name: &'static str,
        help: &'static str,
        counter: Arc<Counter>,
    ) {
        self.counters.push((name, help, counter));
    }

    pub fn register_gauge(&mut self, name: &'static str, help: &'static str, gauge: Arc<Gauge>) {
        self.gauges.push((name, help, gauge));
    }

    pub fn register_histogram(
        &mut self,
        name: &'static str,
        help: &'static str,
        histogram: Arc<LatencyHistogram>,
    ) {
        self.histograms.push((name, help, histogram));
    }

    /// Renders all registered metrics in canonical OpenMetrics / Prometheus text format.
    #[must_use]
    pub fn render_openmetrics_text(&self) -> String {
        let mut out = String::new();

        for (name, help, counter) in &self.counters {
            let _ = writeln!(out, "# TYPE {name} counter");
            let _ = writeln!(out, "# HELP {name} {help}");
            let _ = writeln!(out, "{name} {}", counter.get());
        }

        for (name, help, gauge) in &self.gauges {
            let _ = writeln!(out, "# TYPE {name} gauge");
            let _ = writeln!(out, "# HELP {name} {help}");
            let _ = writeln!(out, "{name} {}", gauge.get());
        }

        for (name, help, hist) in &self.histograms {
            let _ = writeln!(out, "# TYPE {name} histogram");
            let _ = writeln!(out, "# HELP {name} {help}");

            let mut cumulative = 0u64;
            for (i, &le) in hist.buckets.iter().enumerate() {
                cumulative += hist.counts[i].load(Ordering::Relaxed);
                let _ = writeln!(out, "{name}_bucket{{le=\"{le}\"}} {cumulative}");
            }
            let total = hist.count();
            let _ = writeln!(out, "{name}_bucket{{le=\"+Inf\"}} {total}");
            let _ = writeln!(out, "{name}_sum {:.6}", hist.sum_seconds());
            let _ = writeln!(out, "{name}_count {total}");
        }

        out.push_str("# EOF\n");
        out
    }
}
