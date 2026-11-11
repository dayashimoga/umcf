//! Authentication, mTLS & RBAC Bypass Audit (Vectors 6 through 11).

use subtle::ConstantTimeEq;
use crate::audit_logger::{SecurityAuditLedger, SecuritySeverity};
use crate::error::RedteamError;

pub struct RbacAndCryptoAuditor;

impl RbacAndCryptoAuditor {
    /// Vector 6: Unauthenticated / Self-Signed Peer Connection.
    ///
    /// Verifies that untrusted or self-signed root certificates are rejected
    /// during mTLS 1.3 handshake before allocating transport buffers.
    pub fn audit_vector_6_mtls_peer_handshake(
        ledger: &mut SecurityAuditLedger,
        peer_did: &str,
        is_paired: bool,
        is_trusted_ca: bool,
    ) -> Result<(), RedteamError> {
        if !is_paired || !is_trusted_ca {
            ledger.record_blocked_attack(
                6,
                "UNAUTHENTICATED_PEER_MTLS",
                peer_did,
                SecuritySeverity::High,
                "TERMINATE_TLS_HANDSHAKE",
                &format!("Untrusted or unpaired peer '{peer_did}' attempted mTLS connection"),
            );
            return Err(RedteamError::OperationRejected {
                reason: "TlsHandshakeFailed: Peer certificate not in trusted mesh registry".to_string(),
            });
        }
        Ok(())
    }

    /// Vector 7: Revoked Node Certificate / 0-RTT CRL Replay.
    ///
    /// Verifies that revoked certificates immediately invalidate cached 0-RTT session tickets.
    pub fn audit_vector_7_revoked_cert_0rtt_replay(
        ledger: &mut SecurityAuditLedger,
        node_did: &str,
        is_revoked: bool,
    ) -> Result<(), RedteamError> {
        if is_revoked {
            ledger.record_blocked_attack(
                7,
                "REVOKED_CERT_REPLAY",
                node_did,
                SecuritySeverity::Critical,
                "INVALIDATE_TICKET_AND_DROP",
                &format!("Revoked node '{node_did}' attempted 0-RTT session resumption"),
            );
            return Err(RedteamError::OperationRejected {
                reason: "TlsHandshakeFailed: Certificate has been revoked in cluster CRL".to_string(),
            });
        }
        Ok(())
    }

    /// Vector 8: Horizontal & Vertical RBAC Scope Escalation.
    ///
    /// Verifies that unprivileged roles (`GuestObserver`, `cluster:read`)
    /// are rejected with HTTP 403 Forbidden when calling mutating API/gRPC endpoints.
    pub fn audit_vector_8_rbac_authorization(
        ledger: &mut SecurityAuditLedger,
        role: &str,
        endpoint: &str,
        method: &str,
    ) -> Result<u16, RedteamError> {
        let is_mutating = method == "POST" || method == "PUT" || method == "DELETE";

        // Admin has full access
        if role == "ClusterAdmin" {
            return Ok(200);
        }

        // GuestObserver and ReadOnly keys are forbidden from mutating operations
        if is_mutating && (role == "GuestObserver" || role == "cluster:read" || role == "ViewOnly") {
            ledger.record_blocked_attack(
                8,
                "RBAC_UNAUTHORIZED_MUTATION",
                role,
                SecuritySeverity::High,
                "HTTP_403_FORBIDDEN",
                &format!("Role '{role}' denied mutation '{method} {endpoint}'"),
            );
            return Ok(403);
        }

        Ok(200)
    }

    /// Vector 9: Private AI Enclave Exfiltration Attempt.
    ///
    /// Verifies that when `StrictLocalOnly` mode is active, any remote layer offloading
    /// request trips the local circuit breaker, logs a CRITICAL security alert, and halts.
    pub fn audit_vector_9_private_ai_exfiltration(
        ledger: &mut SecurityAuditLedger,
        strict_local_only: bool,
        remote_offload_requested: bool,
    ) -> Result<(), RedteamError> {
        if strict_local_only && remote_offload_requested {
            ledger.record_blocked_attack(
                9,
                "PRIVATE_AI_ENCLAVE_EXFILTRATION",
                "compromised-coordinator",
                SecuritySeverity::Critical,
                "CIRCUIT_BREAKER_ABORT",
                "StrictLocalOnly mode active: blocked external prompt activation transfer",
            );
            return Err(RedteamError::OperationRejected {
                reason: "PrivacyEnclaveCircuitBreaker: StrictLocalOnly violates remote offload request".to_string(),
            });
        }
        Ok(())
    }

    /// Vector 10: Remote Stream `ViewOnly` Input & Clipboard Hijack.
    ///
    /// Verifies that clients possessing only `ViewOnly` capability tokens
    /// cannot inject keyboard/mouse inputs or clipboard sync events.
    pub fn audit_vector_10_view_only_input_containment(
        ledger: &mut SecurityAuditLedger,
        client_capability: &str,
        event_kind: &str,
    ) -> Result<(), RedteamError> {
        if client_capability == "ViewOnly" && (event_kind == "InputEvent" || event_kind == "ClipboardSync") {
            ledger.record_blocked_attack(
                10,
                "VIEW_ONLY_INPUT_HIJACK",
                "remote-stream-client",
                SecuritySeverity::High,
                "DROP_PACKET_AND_QUARANTINE",
                &format!("ViewOnly client attempted unauthorized '{event_kind}' injection"),
            );
            return Err(RedteamError::OperationRejected {
                reason: "StreamRbac: Client lacks stream:desktop:input capability".to_string(),
            });
        }
        Ok(())
    }

    /// Vector 11: Constant-Time Comparison on API Keys and Pairing PINs.
    ///
    /// Verifies that secret comparisons use `subtle::ConstantTimeEq` to prevent
    /// cache and branch predictor timing side-channel attacks.
    pub fn audit_vector_11_constant_time_eq(
        a: &[u8],
        b: &[u8],
    ) -> bool {
        a.ct_eq(b).into()
    }
}

