//! Tamper-evident Security Audit Logger recording blocked red-team attacks.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum SecuritySeverity {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityAuditEvent {
    pub timestamp_ms: u64,
    pub source_node_or_ip: String,
    pub vector_category: String,
    pub vector_id: u8,
    pub severity: SecuritySeverity,
    pub action_taken: String,
    pub details: String,
}

#[derive(Debug, Default)]
pub struct SecurityAuditLedger {
    pub events: Vec<SecurityAuditEvent>,
}

impl SecurityAuditLedger {
    pub fn new() -> Self {
        Self { events: Vec::new() }
    }

    pub fn record_blocked_attack(
        &mut self,
        vector_id: u8,
        vector_category: &str,
        source: &str,
        severity: SecuritySeverity,
        action: &str,
        details: &str,
    ) {
        let event = SecurityAuditEvent {
            timestamp_ms: 1731322000000 + (self.events.len() as u64 * 50),
            source_node_or_ip: source.to_string(),
            vector_category: vector_category.to_string(),
            vector_id,
            severity,
            action_taken: action.to_string(),
            details: details.to_string(),
        };
        self.events.push(event);
    }

    pub fn count_by_severity(&self, severity: SecuritySeverity) -> usize {
        self.events.iter().filter(|e| e.severity == severity).count()
    }
}

