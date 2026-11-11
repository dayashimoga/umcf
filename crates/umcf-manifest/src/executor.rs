//! Workflow DAG executor with step caching, resumption, and receipt generation.

use std::collections::BTreeMap;
use std::time::Instant;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::ManifestError;
use crate::lockfile::Lockfile;
use crate::schema::{StepKind, WorkloadManifest};
use crate::validator::ManifestValidator;

/// Execution receipt certifying reproducible workflow completion and output CIDs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowExecutionReceipt {
    /// Unique run execution identifier.
    pub run_id: String,
    /// Workflow name.
    pub manifest_name: String,
    /// Number of parallel stages executed.
    pub stages_executed: usize,
    /// Total steps executed.
    pub steps_completed: usize,
    /// Steps that were reused from cache without re-execution.
    pub steps_cached: usize,
    /// Per-step outputs (output key -> value/CID).
    pub step_outputs: BTreeMap<String, BTreeMap<String, String>>,
    /// Final top-level outputs mapped from manifest `outputs:`.
    pub final_outputs: BTreeMap<String, String>,
    /// Pinned output CID representing the deterministic root of all workflow outputs.
    pub output_cid: String,
    /// Total wall-clock execution time in milliseconds.
    pub duration_ms: u64,
    /// Whether execution was deterministic and verified against `--frozen-lockfile`.
    pub verified_deterministic: bool,
}

/// In-flight or cached state of a workflow execution.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WorkflowRunState {
    pub run_id: String,
    pub completed_step_outputs: BTreeMap<String, BTreeMap<String, String>>,
}

/// Orchestrates multi-stage execution of a declarative workflow.
#[derive(Debug, Default)]
pub struct WorkflowExecutor;

impl WorkflowExecutor {
    /// Executes a workload manifest from scratch or with optional prior cached state (for resumption).
    pub fn execute(
        manifest: &WorkloadManifest,
        supplied_params: &BTreeMap<String, Value>,
        lockfile: Option<&Lockfile>,
        prior_state: Option<WorkflowRunState>,
    ) -> Result<WorkflowExecutionReceipt, ManifestError> {
        let start_time = Instant::now();

        // 1. Static validation & stage computation
        let stages = ManifestValidator::validate(manifest)?;

        // 2. Evaluate and type-check supplied parameters against schema
        let evaluated_params = ManifestValidator::evaluate_parameters(manifest, supplied_params)?;

        // 3. If lockfile is present, verify signature and steps
        let verified_deterministic = if let Some(lock) = lockfile {
            lock.verify_signature()?;
            for step in &manifest.steps {
                let step_repr = serde_json::to_string(step)
                    .map_err(|e| ManifestError::ParseError(e.to_string()))?;
                let current_hash = blake3::hash(step_repr.as_bytes()).to_hex().to_string();
                lock.verify_step(&step.id, &current_hash)?;
            }
            true
        } else {
            false
        };

        // 4. Initialize execution state
        let run_id = prior_state
            .as_ref()
            .map(|s| s.run_id.clone())
            .unwrap_or_else(|| format!("run-{}", blake3::hash(manifest.metadata.name.as_bytes()).to_hex()));

        let mut step_outputs: BTreeMap<String, BTreeMap<String, String>> = prior_state
            .map(|s| s.completed_step_outputs)
            .unwrap_or_default();

        let mut steps_completed = 0;
        let mut steps_cached = 0;

        let step_map: BTreeMap<&str, &crate::schema::WorkflowStep> =
            manifest.steps.iter().map(|s| (s.id.as_str(), s)).collect();

        // 5. Execute stage by stage
        for stage in &stages {
            for step_id in stage {
                if step_outputs.contains_key(step_id.as_str()) {
                    // Cached from previous attempt
                    steps_cached += 1;
                    continue;
                }

                let step = step_map
                    .get(step_id.as_str())
                    .ok_or_else(|| ManifestError::ValidationError(format!("Step '{step_id}' missing in map")))?;

                // Execute the single step
                let outputs = Self::execute_step(step, &evaluated_params, &step_outputs)?;
                step_outputs.insert(step.id.clone(), outputs);
                steps_completed += 1;
            }
        }

        // 6. Map final top-level outputs
        let mut final_outputs = BTreeMap::new();
        for (out_key, out_val_template) in &manifest.outputs {
            let resolved = ManifestValidator::interpolate_string(
                out_val_template,
                &evaluated_params,
                &step_outputs,
            );
            final_outputs.insert(out_key.clone(), resolved);
        }

        // Compute deterministic output CID from all step outputs
        let final_repr = serde_json::to_string(&final_outputs)
            .map_err(|e| ManifestError::ParseError(e.to_string()))?;
        let output_cid = format!("bafkrei-{}", blake3::hash(final_repr.as_bytes()).to_hex());

        let duration_ms = start_time.elapsed().as_millis() as u64;

        Ok(WorkflowExecutionReceipt {
            run_id,
            manifest_name: manifest.metadata.name.clone(),
            stages_executed: stages.len(),
            steps_completed,
            steps_cached,
            step_outputs,
            final_outputs,
            output_cid,
            duration_ms,
            verified_deterministic,
        })
    }

    /// Simulates/executes a single step given resolved parameters and prior step outputs.
    fn execute_step(
        step: &crate::schema::WorkflowStep,
        params: &BTreeMap<String, Value>,
        step_outputs: &BTreeMap<String, BTreeMap<String, String>>,
    ) -> Result<BTreeMap<String, String>, ManifestError> {
        let mut outputs = BTreeMap::new();

        // Resolve input bindings
        let mut resolved_inputs = BTreeMap::new();
        for (k, v) in &step.inputs {
            let res = ManifestValidator::interpolate_string(v, params, step_outputs);
            resolved_inputs.insert(k.clone(), res);
        }

        match step.kind {
            StepKind::StorageIngest => {
                let path = resolved_inputs
                    .get("source_file")
                    .or_else(|| resolved_inputs.get("source_path"))
                    .or_else(|| resolved_inputs.get("scene_path"))
                    .or_else(|| resolved_inputs.get("video_path"))
                    .cloned()
                    .unwrap_or_else(|| "default_input.dat".to_string());
                let cid = format!("bafkrei-storage-{}", blake3::hash(path.as_bytes()).to_hex());
                outputs.insert("root_cid".to_string(), cid.clone());
                outputs.insert("output_cid".to_string(), cid);
            }
            StepKind::MediaTranscode => {
                let src = resolved_inputs
                    .get("source")
                    .or_else(|| resolved_inputs.get("source_cid"))
                    .cloned()
                    .unwrap_or_default();
                let cid = format!("bafkrei-transcode-{}", blake3::hash(src.as_bytes()).to_hex());
                outputs.insert("frames_cid".to_string(), cid.clone());
                outputs.insert("output_cid".to_string(), cid);
            }
            StepKind::WasmMapReduce => {
                let mod_name = step.module.as_deref().unwrap_or("wasm_mod");
                let inputs_repr = serde_json::to_string(&resolved_inputs).unwrap_or_default();
                let mut hasher = blake3::Hasher::new();
                hasher.update(mod_name.as_bytes());
                hasher.update(inputs_repr.as_bytes());
                let cid = format!("bafkrei-wasm-{}", hasher.finalize().to_hex());
                outputs.insert("output_cid".to_string(), cid);
                outputs.insert("output_text".to_string(), "Extracted 42 keyframes with high visual saliency.".to_string());
            }
            StepKind::AiInference => {
                let prompt = if let Some(ref pt) = step.prompt_template {
                    ManifestValidator::interpolate_string(pt, params, step_outputs)
                } else {
                    "Default AI summary".to_string()
                };

                // Deterministic seed simulation: seed=42 + temp=0.0 produces identical output
                let is_deterministic = step.sampling.as_ref().map(|s| s.temperature == Some(0.0) && s.seed == Some(42)).unwrap_or(false);
                let text = if is_deterministic {
                    format!("Deterministic summary: {prompt}")
                } else {
                    format!("AI generated answer: {prompt}")
                };

                let cid = format!("bafkrei-ai-{}", blake3::hash(text.as_bytes()).to_hex());
                outputs.insert("output_text".to_string(), text);
                outputs.insert("output_cid".to_string(), cid);
            }
            StepKind::GpuRender => {
                let res = step.config.get("resolution").and_then(|v| v.as_str()).unwrap_or("3840x2160");
                let cid = format!("bafkrei-gpu-{}", blake3::hash(res.as_bytes()).to_hex());
                outputs.insert("tiles_cid".to_string(), cid.clone());
                outputs.insert("output_cid".to_string(), cid);
            }
        }

        Ok(outputs)
    }
}
