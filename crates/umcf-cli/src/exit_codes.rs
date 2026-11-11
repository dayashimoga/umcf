// exit_codes.rs
// Deterministic Exit Codes for the `umcf` CLI.

use std::process::ExitCode;

/// Standardized UMCF CLI exit codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CliExitCode {
    /// Command completed successfully (0).
    Ok = 0,
    /// General internal or runtime execution error (1).
    RuntimeError = 1,
    /// CLI argument usage or schema validation error (2).
    UsageError = 2,
    /// Authentication missing, invalid, or permission denied (3).
    AuthError = 3,
    /// Resource quota or capacity exhausted (4).
    QuotaExhausted = 4,
    /// Network connection failure or timeout (5).
    NetworkTimeout = 5,
    /// Graceful interrupt (SIGINT / Ctrl-C) with in-flight cancellation (130).
    SigintInterrupt = 130,
}

impl CliExitCode {
    #[must_use]
    pub fn as_u8(&self) -> u8 {
        *self as u8
    }

    #[must_use]
    pub fn as_i32(&self) -> i32 {
        *self as i32
    }

    #[must_use]
    pub fn into_exit_code(self) -> ExitCode {
        ExitCode::from(self.as_u8())
    }
}
