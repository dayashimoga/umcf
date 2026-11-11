//! # UMCF Security Red-Team — Adversarial Penetration Testing Suite
//!
//! Provides automated exploit verification for sandbox escapes,
//! filesystem traversals, identity forgeries, RBAC escalations,
//! and Private AI prompt exfiltration.

#![forbid(unsafe_code)]

pub mod audit_logger;
pub mod error;
pub mod rbac_and_crypto_audit;
pub mod sandbox_escape;

pub use audit_logger::{SecurityAuditEvent, SecurityAuditLedger, SecuritySeverity};
pub use error::RedteamError;
pub use rbac_and_crypto_audit::RbacAndCryptoAuditor;
pub use sandbox_escape::SandboxEscapeAuditor;

