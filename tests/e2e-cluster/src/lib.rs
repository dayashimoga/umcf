//! `e2e-cluster`
//!
//! Multi-OS Cluster Integration Suite, Cross-Platform Wire Normalization, and Architecture Determinism
//! for Universal Mobile Compute Fabric.

#![forbid(unsafe_code)]

pub mod error;
pub mod multi_os_suite;
pub mod platform_compat;

// Re-exports
pub use error::PlatformError;
pub use multi_os_suite::{
    MultiOsClusterHarness, MultiOsNodeProfile, ScenarioResult, TargetArch, TargetOs,
};
pub use platform_compat::{
    CaseCollisionDetector, PageAlignmentValidator, VirtualPathNormalizer, PAGE_SIZE_16KB,
    PAGE_SIZE_4KB,
};

