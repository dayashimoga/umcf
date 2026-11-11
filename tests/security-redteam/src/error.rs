//! Error types for Red-Team Penetration Testing and Sandbox Containment.

use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum RedteamError {
    #[error("CRITICAL: Sandbox containment failed for Vector {vector_id} ('{attack_name}'): {reason}")]
    ContainmentFailed {
        vector_id: u8,
        attack_name: String,
        reason: String,
    },

    #[error("RBAC bypass detected: role '{role}' granted unauthorized access to endpoint '{endpoint}'")]
    UnauthorizedAccessGranted {
        endpoint: String,
        role: String,
    },

    #[error("Cryptographic / side-channel vulnerability detected in '{secret_type}': {reason}")]
    SideChannelVulnerability {
        secret_type: String,
        reason: String,
    },

    #[error("Security audit alert was suppressed or missing for Vector {vector_id}")]
    SecurityAlertSuppressed {
        vector_id: u8,
    },

    #[error("Operation rejected cleanly: {reason}")]
    OperationRejected {
        reason: String,
    },
}

