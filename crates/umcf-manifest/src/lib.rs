//! `umcf-manifest`
//!
//! Declarative Workload Manifests & Reproducible Job Templates for Universal Mobile Compute Fabric.
//! Supports `apiVersion: umcf.io/v1` in YAML and TOML, Kahn's DAG cycle detection, variable interpolation,
//! cryptographic lockfile verification (`umcf.lock`), and built-in template catalogs.

#![forbid(unsafe_code)]

pub mod error;
pub mod executor;
pub mod lockfile;
pub mod schema;
pub mod templates;
pub mod validator;

// Re-exports
pub use error::ManifestError;
pub use executor::{WorkflowExecutionReceipt, WorkflowExecutor, WorkflowRunState};
pub use lockfile::{LockedArtifact, LockedStep, Lockfile};
pub use schema::{
    ManifestMetadata, ParameterDef, ParameterType, PrivacyConfig, PrivacyMode, SamplingConfig,
    StepKind, StepResources, WorkflowStep, WorkloadManifest, SUPPORTED_API_VERSION, WORKFLOW_KIND,
};
pub use templates::{TemplateCatalog, TemplateSummary};
pub use validator::ManifestValidator;
