//! Structured OpenTelemetry tracing, metrics, sensitive data redaction, and standardized error taxonomy for UMCF.
//! Adheres strictly to `#![forbid(unsafe_code)]` and zero-placeholder invariants.

pub mod error;
pub mod metrics;
pub mod tracing_init;

pub use error::{
    all_error_taxonomy_entries, ErrorCatalogEntry, ErrorCode, ProblemDetails, Retryability,
    UmcfError,
};
pub use metrics::{Counter, Gauge, LatencyHistogram, MetricsRegistry};
pub use tracing_init::{
    extract_traceparent_from_envelope, inject_traceparent_into_envelope, Redacted,
};

use std::fmt;
use umcf_core::NodeId;
use umcf_proto::compute_fnv1a_64;

/// W3C-compliant 128-bit Trace ID and 64-bit Span ID context.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TraceContext {
    pub trace_id_hex: String,
    pub span_id_hex: String,
    pub parent_span_id_hex: Option<String>,
    pub sampled: bool,
}

impl TraceContext {
    /// Generates a deterministic `TraceContext` from an execution seed and sequence index.
    #[must_use]
    pub fn new(seed: &[u8], sequence_id: u64) -> Self {
        let hi = compute_fnv1a_64(seed);
        let mut seed_ext = seed.to_vec();
        seed_ext.extend_from_slice(&sequence_id.to_be_bytes());
        let lo = compute_fnv1a_64(&seed_ext);
        let span = compute_fnv1a_64(&lo.to_be_bytes());

        Self {
            trace_id_hex: format!("{hi:016x}{lo:016x}"),
            span_id_hex: format!("{span:016x}"),
            parent_span_id_hex: None,
            sampled: true,
        }
    }

    /// Formats the context into a standard W3C `traceparent` header string:
    /// `00-<trace_id>-<span_id>-<flags>`
    #[must_use]
    pub fn to_w3c_traceparent(&self) -> String {
        let flags = if self.sampled { "01" } else { "00" };
        format!("00-{}-{}-{}", self.trace_id_hex, self.span_id_hex, flags)
    }

    /// Parses a standard W3C `traceparent` header.
    pub fn from_w3c_traceparent(header: &str) -> Result<Self, TelemetryError> {
        let parts: Vec<&str> = header.split('-').collect();
        if parts.len() != 4 || parts[0] != "00" {
            return Err(TelemetryError::InvalidTraceparentHeader(header.to_string()));
        }
        let trace_id_hex = parts[1];
        let span_id_hex = parts[2];
        let flags = parts[3];

        if trace_id_hex.len() != 32 || span_id_hex.len() != 16 {
            return Err(TelemetryError::InvalidTraceparentHeader(header.to_string()));
        }

        let sampled = flags == "01";
        Ok(Self {
            trace_id_hex: trace_id_hex.to_string(),
            span_id_hex: span_id_hex.to_string(),
            parent_span_id_hex: None,
            sampled,
        })
    }

    /// Creates a child span context linked to `self` as parent.
    #[must_use]
    pub fn child_span(&self, step_seed: &[u8]) -> Self {
        let span = compute_fnv1a_64(step_seed);
        Self {
            trace_id_hex: self.trace_id_hex.clone(),
            span_id_hex: format!("{span:016x}"),
            parent_span_id_hex: Some(self.span_id_hex.clone()),
            sampled: self.sampled,
        }
    }
}

/// Telemetry metric kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MetricKind {
    CpuUtilizationPercent,
    MemoryAllocatedMb,
    TokensPerSecond,
    NetworkTransferredBytes,
    TemperatureCelsius,
    ActiveTasksCount,
}

/// A timestamped metric measurement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetricMeasurement {
    pub node_id: NodeId,
    pub kind: MetricKind,
    pub value: u64,
    pub timestamp_unix_secs: u64,
}

/// Structured log level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl fmt::Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Trace => write!(f, "TRACE"),
            Self::Debug => write!(f, "DEBUG"),
            Self::Info => write!(f, "INFO"),
            Self::Warn => write!(f, "WARN"),
            Self::Error => write!(f, "ERROR"),
        }
    }
}

/// A structured log event with correlation ID.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructuredLogEvent {
    pub level: LogLevel,
    pub message: String,
    pub trace_context: Option<TraceContext>,
    pub node_id: NodeId,
    pub timestamp_unix_secs: u64,
}

/// Telemetry errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TelemetryError {
    InvalidTraceparentHeader(String),
}

impl fmt::Display for TelemetryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidTraceparentHeader(h) => {
                write!(f, "invalid W3C traceparent header: '{h}'")
            }
        }
    }
}

impl std::error::Error for TelemetryError {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;
    use std::sync::Arc;
    use umcf_proto::{MessageKind, PlaneKind, WireEnvelope};

    #[test]
    fn unit_w3c_traceparent_roundtrip_and_child_span() {
        let parent = TraceContext::new(b"initial-task-seed-01", 100);
        assert_eq!(parent.trace_id_hex.len(), 32);
        assert_eq!(parent.span_id_hex.len(), 16);
        assert!(parent.sampled);

        let header = parent.to_w3c_traceparent();
        assert!(header.starts_with("00-"));

        let parsed = TraceContext::from_w3c_traceparent(&header).expect("parse traceparent");
        assert_eq!(parent.trace_id_hex, parsed.trace_id_hex);
        assert_eq!(parent.span_id_hex, parsed.span_id_hex);
        assert_eq!(parent.sampled, parsed.sampled);

        let child = parent.child_span(b"sub-step-execution");
        assert_eq!(child.trace_id_hex, parent.trace_id_hex);
        assert_ne!(child.span_id_hex, parent.span_id_hex);
        assert_eq!(child.parent_span_id_hex, Some(parent.span_id_hex));
    }

    #[test]
    fn unit_error_taxonomy_mappings_and_problem_details() {
        let err = UmcfError::new(ErrorCode::E4002ThermalCutoff, "Thermal headroom exceeded")
            .with_detail("Core temp reached 85C exceeding 75C safe threshold")
            .with_trace_id("0123456789abcdef0123456789abcdef");

        assert_eq!(err.code(), ErrorCode::E4002ThermalCutoff);
        assert_eq!(err.http_status(), 503);
        assert_eq!(err.grpc_status(), 14);
        assert!(matches!(err.retryability(), Retryability::After(_)));

        let pd = err.to_problem_details();
        assert_eq!(pd.type_uri, "urn:umcf:error:E4002");
        assert_eq!(pd.status, 503);
        assert_eq!(pd.error_code, "E4002");
        assert_eq!(
            pd.instance,
            Some("urn:umcf:trace:0123456789abcdef0123456789abcdef".into())
        );

        // Serialize to JSON RFC-9457
        let json = serde_json::to_string(&pd).expect("serialize problem details");
        assert!(json.contains("\"type\":\"urn:umcf:error:E4002\""));
        assert!(json.contains("\"status\":503"));
    }

    #[test]
    fn unit_lock_free_metrics_and_openmetrics_export() {
        let counter = Arc::new(Counter::new());
        counter.inc();
        counter.inc_by(41);
        assert_eq!(counter.get(), 42);

        let gauge = Arc::new(Gauge::new());
        gauge.set(10);
        gauge.inc();
        gauge.dec();
        assert_eq!(gauge.get(), 10);

        let hist = Arc::new(LatencyHistogram::new(vec![0.001, 0.005, 0.010, 0.050]));
        hist.observe(0.002);
        hist.observe(0.004);
        hist.observe(0.008);
        assert_eq!(hist.count(), 3);

        let mut reg = MetricsRegistry::new();
        reg.register_counter("umcf_tasks_total", "Total tasks executed", counter);
        reg.register_gauge("umcf_active_nodes", "Active connected nodes", gauge);
        reg.register_histogram("umcf_task_latency_seconds", "Task latency", hist);

        let text = reg.render_openmetrics_text();
        assert!(text.contains("# TYPE umcf_tasks_total counter"));
        assert!(text.contains("umcf_tasks_total 42"));
        assert!(text.contains("# TYPE umcf_active_nodes gauge"));
        assert!(text.contains("umcf_active_nodes 10"));
        assert!(text.contains("umcf_task_latency_seconds_bucket"));
        assert!(text.ends_with("# EOF\n"));
    }

    #[test]
    fn integration_trace_propagation_across_wire_envelope() {
        let trace_ctx = TraceContext::new(b"workload-span-root", 42);
        let mut env = WireEnvelope::new(
            PlaneKind::ControlPlane,
            MessageKind::ScheduleTask,
            1,
            "did:umcf:trace-node-01",
            vec![1, 2, 3],
        )
        .expect("envelope");

        // Inject trace context
        inject_traceparent_into_envelope(&mut env, &trace_ctx);

        // Encode to wire bytes
        let wire_bytes = env.to_bytes();
        let decoded_env = WireEnvelope::from_bytes(&wire_bytes).expect("decode wire bytes");

        // Extract and verify exact match
        let extracted_ctx = extract_traceparent_from_envelope(&decoded_env);
        assert_eq!(trace_ctx.trace_id_hex, extracted_ctx.trace_id_hex);
        assert_eq!(trace_ctx.span_id_hex, extracted_ctx.span_id_hex);
    }

    #[test]
    fn integration_export_error_taxonomy_catalog() {
        let entries = all_error_taxonomy_entries();
        assert_eq!(entries.len(), 27);

        let json = serde_json::to_string_pretty(&entries).expect("catalog json");

        let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        let root_artifacts = manifest_dir
            .parent()
            .and_then(|p| p.parent())
            .map(|p| p.join("artifacts").join("evidence").join("stage-01"))
            .unwrap_or_else(|| std::path::PathBuf::from("artifacts/evidence/stage-01"));

        fs::create_dir_all(&root_artifacts).expect("create evidence dir");
        let catalog_path = root_artifacts.join("error_taxonomy_catalog.json");
        fs::write(&catalog_path, json).expect("write catalog");
        assert!(catalog_path.exists());
    }

    #[test]
    fn simulation_sensitive_secret_redaction_leak_prevention() {
        let raw_secret_key = "SEC_PRIVATE_KEY_99998888777766665555444433332222";
        let secret = Redacted::new(raw_secret_key.to_string());

        // 1. Display must redact
        let display_str = format!("{secret}");
        assert_eq!(display_str, "[REDACTED]");
        assert!(!display_str.contains(raw_secret_key));

        // 2. Debug must redact
        let debug_str = format!("{secret:?}");
        assert_eq!(debug_str, "[REDACTED_SECRET]");
        assert!(!debug_str.contains(raw_secret_key));

        // 3. JSON serialization must redact
        let json_str = serde_json::to_string(&secret).expect("json");
        assert_eq!(json_str, "\"[REDACTED_SECRET]\"");
        assert!(!json_str.contains(raw_secret_key));

        // 4. Secret can still be accessed explicitly via expose_secret()
        assert_eq!(secret.expose_secret(), raw_secret_key);
    }
}
