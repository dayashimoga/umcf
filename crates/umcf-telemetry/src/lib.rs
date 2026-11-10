//! Structured OpenTelemetry tracing, metrics, and correlation IDs for UMCF.
//! Adheres strictly to `#![forbid(unsafe_code)]` and zero-placeholder invariants.

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
    fn unit_invalid_traceparent_rejected() {
        assert!(TraceContext::from_w3c_traceparent("invalid").is_err());
        assert!(TraceContext::from_w3c_traceparent("01-tooshort-span-00").is_err());
        assert!(TraceContext::from_w3c_traceparent("00-1234-5678-01").is_err());
    }

    #[test]
    fn unit_structured_telemetry_measurements() {
        let node_id = NodeId::from_public_key_bytes(b"telemetry-node-key-seed-01").expect("id");
        let metric = MetricMeasurement {
            node_id: node_id.clone(),
            kind: MetricKind::TokensPerSecond,
            value: 48,
            timestamp_unix_secs: 1_700_000_100,
        };
        assert_eq!(metric.kind, MetricKind::TokensPerSecond);
        assert_eq!(metric.value, 48);

        let log = StructuredLogEvent {
            level: LogLevel::Info,
            message: "Node joined mesh cluster".into(),
            trace_context: None,
            node_id,
            timestamp_unix_secs: 1_700_000_100,
        };
        assert_eq!(log.level.to_string(), "INFO");
    }
}
