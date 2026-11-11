//! Exhaustive structured error types for `umcf-manifest`.

use thiserror::Error;

/// Structured manifest parsing, validation, lockfile, and execution errors.
#[derive(Debug, Error)]
pub enum ManifestError {
    /// Deserialization failed from YAML, TOML, or JSON format.
    #[error("Manifest parse error: {0}")]
    ParseError(String),

    /// General semantic validation failure.
    #[error("Manifest validation error: {0}")]
    ValidationError(String),

    /// A cycle exists in the step dependency graph.
    #[error("Topological cycle detected in step dependencies: {cycle:?}")]
    CyclicDependency { cycle: Vec<String> },

    /// An undefined parameter was referenced.
    #[error("Undefined parameter referenced: '{name}' in step '{step_id}'")]
    UndefinedParameter { name: String, step_id: String },

    /// An undefined step output was referenced.
    #[error("Undefined step output referenced: '{step_id}.{output_key}'")]
    UndefinedStepOutput { step_id: String, output_key: String },

    /// A step references another step's output without declaring it in `depends_on`.
    #[error("Missing dependency: step '{step_id}' references output of step '{dependency_id}', but '{dependency_id}' is not in 'depends_on'")]
    MissingDependency { step_id: String, dependency_id: String },

    /// Type mismatch in parameter value or variable interpolation.
    #[error("Type mismatch: parameter '{param_name}' expects {expected}, found {actual}")]
    TypeMismatch { param_name: String, expected: String, actual: String },

    /// Duplicate step identifier within manifest.
    #[error("Duplicate step ID: '{step_id}'")]
    DuplicateStepId { step_id: String },

    /// Content hash mismatch under `--frozen-lockfile`.
    #[error("Lockfile mismatch under frozen lockfile: step '{step_id}' artifact hash changed (expected: {expected}, found: {actual})")]
    LockfileMismatch { step_id: String, expected: String, actual: String },

    /// Execution required frozen lockfile, but no lockfile was present.
    #[error("Missing lockfile: workflow execution requires frozen lockfile but no valid umcf.lock was provided")]
    MissingLockfile,

    /// Ed25519 signature verification on lockfile failed.
    #[error("Signature verification failed for lockfile: {0}")]
    InvalidSignature(String),

    /// Requested built-in job template does not exist.
    #[error("Template not found: '{name}'")]
    TemplateNotFound { name: String },

    /// Standard I/O error.
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    /// Workflow execution error.
    #[error("Execution error: {0}")]
    ExecutionError(String),
}
