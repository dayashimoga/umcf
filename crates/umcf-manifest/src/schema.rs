//! Workload manifest schema for `apiVersion: umcf.io/v1`.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

use crate::error::ManifestError;

/// Supported API versions for UMCF declarative manifests.
pub const SUPPORTED_API_VERSION: &str = "umcf.io/v1";

/// Expected Kind for distributed workflow manifests.
pub const WORKFLOW_KIND: &str = "DistributedWorkflow";

/// Top-level declarative workload manifest.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkloadManifest {
    /// API version identifier (e.g., `umcf.io/v1`).
    #[serde(rename = "apiVersion")]
    pub api_version: String,

    /// Resource kind (e.g., `DistributedWorkflow`).
    pub kind: String,

    /// Workload metadata.
    pub metadata: ManifestMetadata,

    /// Input parameters schema with validation rules and defaults.
    #[serde(default)]
    pub parameters: BTreeMap<String, ParameterDef>,

    /// Node execution privacy and confidentiality governance.
    #[serde(default)]
    pub privacy: Option<PrivacyConfig>,

    /// Ordered or DAG step definitions.
    pub steps: Vec<WorkflowStep>,

    /// Key-value mappings exposing final workflow outputs (e.g., `${steps.<id>.<key>}`).
    #[serde(default)]
    pub outputs: BTreeMap<String, String>,
}

/// Metadata identifying a workflow.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ManifestMetadata {
    /// Machine-readable identifier for the workflow.
    pub name: String,

    /// Semantic version (e.g. "1.0.0").
    pub version: String,

    /// Optional author or maintainer DID/email.
    #[serde(default)]
    pub author: Option<String>,

    /// Human-readable description of workload pipeline.
    #[serde(default)]
    pub description: Option<String>,
}

/// Permitted primitive parameter types for workload inputs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ParameterType {
    String,
    Int,
    Float,
    Bool,
    Path,
}

/// Parameter definition in manifest.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ParameterDef {
    /// Primitive parameter type.
    #[serde(rename = "type")]
    pub param_type: ParameterType,

    /// Whether caller must supply this parameter.
    #[serde(default)]
    pub required: bool,

    /// Fallback default value when omitted.
    #[serde(default)]
    pub default: Option<Value>,

    /// Human-readable parameter explanation.
    #[serde(default)]
    pub description: Option<String>,
}

/// Privacy modes governing peer node participation and data plane enclaves.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum PrivacyMode {
    /// Workload may only execute on local device, never offloaded.
    StrictLocal,
    /// Workload may be offloaded to verified devices enrolled in local cluster.
    #[default]
    TrustedDevices,
    /// Workload may run on untrusted external workers via encrypted WASM/enclave.
    ClusterEncrypted,
}

/// Privacy configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PrivacyConfig {
    pub mode: PrivacyMode,
}

/// Supported execution step kinds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StepKind {
    #[serde(rename = "media.transcode")]
    MediaTranscode,
    #[serde(rename = "wasm.mapreduce")]
    WasmMapReduce,
    #[serde(rename = "ai.inference")]
    AiInference,
    #[serde(rename = "gpu.render")]
    GpuRender,
    #[serde(rename = "storage.ingest")]
    StorageIngest,
}

/// Hardware resource bounds for a workflow step.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct StepResources {
    #[serde(default)]
    pub max_memory_mb: Option<u64>,
    #[serde(default)]
    pub timeout_sec: Option<u64>,
    #[serde(default)]
    pub cpu_cores: Option<u32>,
    #[serde(default)]
    pub vram_mb: Option<u64>,
}

/// LLM sampling parameters for deterministic or probabilistic inference.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct SamplingConfig {
    #[serde(default)]
    pub temperature: Option<f32>,
    #[serde(default)]
    pub seed: Option<u64>,
    #[serde(default)]
    pub top_p: Option<f32>,
}

/// Single step definition in a DAG workflow.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkflowStep {
    /// Unique identifier for this step within the manifest.
    pub id: String,

    /// Execution engine kind.
    pub kind: StepKind,

    /// Step IDs that must successfully finish before this step begins.
    #[serde(default)]
    pub depends_on: Vec<String>,

    /// Named input variables or parameters.
    #[serde(default)]
    pub inputs: BTreeMap<String, String>,

    /// Engine-specific configurations.
    #[serde(default)]
    pub config: BTreeMap<String, Value>,

    /// Optional resource constraints.
    #[serde(default)]
    pub resources: Option<StepResources>,

    /// Optional WASM module identifier (CID or path).
    #[serde(default)]
    pub module: Option<String>,

    /// Optional GGUF model identifier (CID or path).
    #[serde(default)]
    pub model: Option<String>,

    /// Optional prompt template string for AI inference steps.
    #[serde(default)]
    pub prompt_template: Option<String>,

    /// Optional sampling configuration.
    #[serde(default)]
    pub sampling: Option<SamplingConfig>,
}

impl WorkloadManifest {
    /// Parses a manifest from a YAML string.
    pub fn from_yaml_str(yaml: &str) -> Result<Self, ManifestError> {
        serde_yaml::from_str(yaml).map_err(|e| ManifestError::ParseError(e.to_string()))
    }

    /// Parses a manifest from a TOML string.
    pub fn from_toml_str(toml_str: &str) -> Result<Self, ManifestError> {
        toml::from_str(toml_str).map_err(|e| ManifestError::ParseError(e.to_string()))
    }

    /// Parses a manifest from a JSON string.
    pub fn from_json_str(json_str: &str) -> Result<Self, ManifestError> {
        serde_json::from_str(json_str).map_err(|e| ManifestError::ParseError(e.to_string()))
    }

    /// Serializes the manifest into a formatted YAML string.
    pub fn to_yaml_str(&self) -> Result<String, ManifestError> {
        serde_yaml::to_string(self).map_err(|e| ManifestError::ParseError(e.to_string()))
    }

    /// Serializes the manifest into a TOML string.
    pub fn to_toml_str(&self) -> Result<String, ManifestError> {
        toml::to_string_pretty(self).map_err(|e| ManifestError::ParseError(e.to_string()))
    }

    /// Serializes the manifest into a formatted JSON string.
    pub fn to_json_str(&self) -> Result<String, ManifestError> {
        serde_json::to_string_pretty(self).map_err(|e| ManifestError::ParseError(e.to_string()))
    }
}
