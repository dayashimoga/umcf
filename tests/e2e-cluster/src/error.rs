//! Exhaustive structured error types for cross-platform cluster testing.

use thiserror::Error;

/// Structured errors encountered in cross-platform normalization and cluster integration.
#[derive(Debug, Error)]
pub enum PlatformError {
    /// A path intended for wire transmission violated cross-platform canonical rules.
    #[error("Non-canonical wire path '{path}' rejected: {reason}")]
    InvalidWirePath { path: String, reason: String },

    /// Two manifest file paths collide when evaluated on a case-insensitive filesystem (NTFS/APFS).
    #[error("Case collision detected between '{path_a}' and '{path_b}' (unsafe on case-insensitive filesystems)")]
    CaseCollisionDetected { path_a: String, path_b: String },

    /// Memory address or buffer offset is not aligned to the target OS page size (e.g. 16 KiB on Android 15/Apple Silicon).
    #[error("Page alignment violation: address {address:#x} (offset {offset}) is not aligned to {required_page_size_bytes} bytes")]
    PageAlignmentViolation {
        address: usize,
        offset: usize,
        required_page_size_bytes: usize,
    },

    /// Multi-OS integration scenario failure.
    #[error("Scenario '{scenario}' failed on target '{target_os}': {reason}")]
    ScenarioFailed {
        scenario: String,
        target_os: String,
        reason: String,
    },

    /// Determinism divergence across architectures.
    #[error("Determinism failure: hash on {arch_a} ({hash_a}) != hash on {arch_b} ({hash_b})")]
    DeterminismDivergence {
        arch_a: String,
        hash_a: String,
        arch_b: String,
        hash_b: String,
    },
}
