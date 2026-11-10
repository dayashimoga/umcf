//! Structured tracing initialization, sensitive secret redaction, and W3C TraceContext propagation.
//! Adheres strictly to `#![forbid(unsafe_code)]` and zero-placeholder invariants.

use serde::{Serialize, Serializer};
use std::fmt;
use std::ops::Deref;
use umcf_proto::WireEnvelope;

use crate::TraceContext;

/// Wrapper that protects sensitive values from accidental exposure in logs, debug dumps, or serialization.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Redacted<T>(pub T);

impl<T> Redacted<T> {
    #[must_use]
    pub const fn new(value: T) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn expose_secret(&self) -> &T {
        &self.0
    }
}

impl<T> Deref for Redacted<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> fmt::Debug for Redacted<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[REDACTED_SECRET]")
    }
}

impl<T> fmt::Display for Redacted<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[REDACTED]")
    }
}

impl<T> Serialize for Redacted<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str("[REDACTED_SECRET]")
    }
}

/// Injects a W3C `TraceContext` into a `WireEnvelope`.
pub fn inject_traceparent_into_envelope(env: &mut WireEnvelope, ctx: &TraceContext) {
    if let Ok(trace_val) = u128::from_str_radix(&ctx.trace_id_hex, 16) {
        env.message_id = umcf_proto::MessageId(trace_val);
    }
    if let Ok(span_val) = u64::from_str_radix(&ctx.span_id_hex, 16) {
        env.sequence_id = span_val;
    }
}

/// Extracts a W3C `TraceContext` from a `WireEnvelope`.
#[must_use]
pub fn extract_traceparent_from_envelope(env: &WireEnvelope) -> TraceContext {
    TraceContext {
        trace_id_hex: format!("{:032x}", env.message_id.0),
        span_id_hex: format!("{:016x}", env.sequence_id),
        parent_span_id_hex: None,
        sampled: true,
    }
}
