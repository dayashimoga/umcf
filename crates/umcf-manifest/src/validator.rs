//! Strict static validator and DAG analyzer for UMCF workload manifests.

use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use serde_json::Value;

use crate::error::ManifestError;
use crate::schema::{ParameterType, WorkloadManifest, SUPPORTED_API_VERSION, WORKFLOW_KIND};

/// Validates a workload manifest and performs topological DAG dependency analysis.
#[derive(Debug, Default)]
pub struct ManifestValidator;

impl ManifestValidator {
    /// Performs exhaustive static validation against a workload manifest.
    pub fn validate(manifest: &WorkloadManifest) -> Result<Vec<Vec<String>>, ManifestError> {
        // 1. Verify API Version and Kind
        if manifest.api_version != SUPPORTED_API_VERSION {
            return Err(ManifestError::ValidationError(format!(
                "Unsupported apiVersion: '{}'. Expected: '{}'",
                manifest.api_version, SUPPORTED_API_VERSION
            )));
        }
        if manifest.kind != WORKFLOW_KIND {
            return Err(ManifestError::ValidationError(format!(
                "Unsupported kind: '{}'. Expected: '{}'",
                manifest.kind, WORKFLOW_KIND
            )));
        }

        // 2. Validate Step ID Uniqueness
        let mut step_ids = HashSet::new();
        for step in &manifest.steps {
            if !step_ids.insert(&step.id) {
                return Err(ManifestError::DuplicateStepId {
                    step_id: step.id.clone(),
                });
            }
        }

        // 3. Validate Step References and Dependency Consistency
        let step_map: HashMap<&str, &crate::schema::WorkflowStep> =
            manifest.steps.iter().map(|s| (s.id.as_str(), s)).collect();

        for step in &manifest.steps {
            for dep in &step.depends_on {
                if !step_map.contains_key(dep.as_str()) {
                    return Err(ManifestError::ValidationError(format!(
                        "Step '{}' specifies non-existent dependency: '{}'",
                        step.id, dep
                    )));
                }
            }
        }

        // 4. DAG Cycle Detection and Topological Stage Grouping (Kahn's Algorithm)
        let stages = Self::compute_topological_stages(&manifest.steps)?;

        // 5. Validate Variable Interpolations (${params.*} and ${steps.*})
        Self::validate_interpolations(manifest, &step_map)?;

        Ok(stages)
    }

    /// Computes parallel topological stages using Kahn's algorithm.
    /// Returns `Vec<Vec<String>>` where each inner vector contains step IDs that can execute concurrently.
    pub fn compute_topological_stages(
        steps: &[crate::schema::WorkflowStep],
    ) -> Result<Vec<Vec<String>>, ManifestError> {
        let mut in_degree: HashMap<&str, usize> = HashMap::new();
        let mut adj: HashMap<&str, Vec<&str>> = HashMap::new();

        for step in steps {
            in_degree.entry(&step.id).or_insert(0);
            for dep in &step.depends_on {
                adj.entry(dep.as_str()).or_default().push(&step.id);
                *in_degree.entry(&step.id).or_insert(0) += 1;
            }
        }

        let mut queue: VecDeque<&str> = in_degree
            .iter()
            .filter(|&(_, &deg)| deg == 0)
            .map(|(&id, _)| id)
            .collect();

        let mut stages = Vec::new();
        let mut visited_count = 0;

        while !queue.is_empty() {
            let stage_size = queue.len();
            let mut current_stage = Vec::new();

            for _ in 0..stage_size {
                if let Some(curr) = queue.pop_front() {
                    current_stage.push(curr.to_string());
                    visited_count += 1;

                    if let Some(neighbors) = adj.get(curr) {
                        for &next in neighbors {
                            if let Some(deg) = in_degree.get_mut(next) {
                                *deg -= 1;
                                if *deg == 0 {
                                    queue.push_back(next);
                                }
                            }
                        }
                    }
                }
            }

            current_stage.sort();
            stages.push(current_stage);
        }

        if visited_count < steps.len() {
            // Find cycle members
            let cycle: Vec<String> = in_degree
                .into_iter()
                .filter(|&(_, deg)| deg > 0)
                .map(|(id, _)| id.to_string())
                .collect();
            return Err(ManifestError::CyclicDependency { cycle });
        }

        Ok(stages)
    }

    /// Validates all `${params.<name>}` and `${steps.<id>.<output>}` variable references.
    fn validate_interpolations(
        manifest: &WorkloadManifest,
        step_map: &HashMap<&str, &crate::schema::WorkflowStep>,
    ) -> Result<(), ManifestError> {
        for step in &manifest.steps {
            let depends_set: HashSet<&str> = step.depends_on.iter().map(|s| s.as_str()).collect();

            // Check inputs
            for (key, template_val) in &step.inputs {
                Self::check_template_string(
                    template_val,
                    manifest,
                    step_map,
                    &step.id,
                    &depends_set,
                    key,
                )?;
            }

            // Check prompt_template if present
            if let Some(ref prompt) = step.prompt_template {
                Self::check_template_string(
                    prompt,
                    manifest,
                    step_map,
                    &step.id,
                    &depends_set,
                    "prompt_template",
                )?;
            }
        }

        // Check top-level outputs
        for (out_key, out_val) in &manifest.outputs {
            let empty_deps = HashSet::new();
            Self::check_template_string(
                out_val,
                manifest,
                step_map,
                "outputs",
                &empty_deps,
                out_key,
            )?;
        }

        Ok(())
    }

    fn check_template_string(
        text: &str,
        manifest: &WorkloadManifest,
        step_map: &HashMap<&str, &crate::schema::WorkflowStep>,
        current_step: &str,
        depends_on: &HashSet<&str>,
        field_name: &str,
    ) -> Result<(), ManifestError> {
        let mut rest = text;
        while let Some(start) = rest.find("${") {
            let remainder = &rest[start + 2..];
            if let Some(end) = remainder.find('}') {
                let expr = &remainder[..end].trim();

                if let Some(param_name) = expr.strip_prefix("params.") {
                    if !manifest.parameters.contains_key(param_name) {
                        return Err(ManifestError::UndefinedParameter {
                            name: param_name.to_string(),
                            step_id: format!("{current_step}.{field_name}"),
                        });
                    }
                } else if let Some(step_expr) = expr.strip_prefix("steps.") {
                    let parts: Vec<&str> = step_expr.split('.').collect();
                    if parts.len() < 2 {
                        return Err(ManifestError::ValidationError(format!(
                            "Malformed step output reference '${{{expr}}}' in '{current_step}.{field_name}'"
                        )));
                    }
                    let ref_step_id = parts[0];
                    let ref_output = parts[1];

                    if !step_map.contains_key(ref_step_id) {
                        return Err(ManifestError::UndefinedStepOutput {
                            step_id: ref_step_id.to_string(),
                            output_key: ref_output.to_string(),
                        });
                    }

                    // If referenced from a step (not top-level outputs), verify it is declared in depends_on
                    if current_step != "outputs" && !depends_on.contains(ref_step_id) {
                        return Err(ManifestError::MissingDependency {
                            step_id: current_step.to_string(),
                            dependency_id: ref_step_id.to_string(),
                        });
                    }
                } else {
                    return Err(ManifestError::ValidationError(format!(
                        "Unrecognized expression '${{{expr}}}' in '{current_step}.{field_name}'. Expected '${{params.*}}' or '${{steps.*}}'"
                    )));
                }

                rest = &remainder[end + 1..];
            } else {
                return Err(ManifestError::ValidationError(format!(
                    "Unclosed '${{' interpolation in '{current_step}.{field_name}'"
                )));
            }
        }
        Ok(())
    }

    /// Evaluates and type-checks concrete input parameters against the manifest schema,
    /// populating defaults where parameters are omitted.
    pub fn evaluate_parameters(
        manifest: &WorkloadManifest,
        supplied_params: &BTreeMap<String, Value>,
    ) -> Result<BTreeMap<String, Value>, ManifestError> {
        let mut evaluated = BTreeMap::new();

        for (name, def) in &manifest.parameters {
            let val = match supplied_params.get(name) {
                Some(v) => v.clone(),
                None => match &def.default {
                    Some(default_val) => default_val.clone(),
                    None => {
                        if def.required {
                            return Err(ManifestError::ValidationError(format!(
                                "Required parameter '{name}' was not supplied"
                            )));
                        } else {
                            Value::Null
                        }
                    }
                },
            };

            // Type check
            if !val.is_null() {
                Self::validate_param_type(name, &val, def.param_type)?;
            }

            evaluated.insert(name.clone(), val);
        }

        Ok(evaluated)
    }

    fn validate_param_type(
        param_name: &str,
        val: &Value,
        expected: ParameterType,
    ) -> Result<(), ManifestError> {
        let matches = match expected {
            ParameterType::String => val.is_string(),
            ParameterType::Path => val.is_string(),
            ParameterType::Int => val.is_i64() || val.is_u64(),
            ParameterType::Float => val.is_f64() || val.is_i64() || val.is_u64(),
            ParameterType::Bool => val.is_boolean(),
        };

        if !matches {
            return Err(ManifestError::TypeMismatch {
                param_name: param_name.to_string(),
                expected: format!("{expected:?}"),
                actual: format!("{val}"),
            });
        }
        Ok(())
    }

    /// Interpolates concrete parameter values into a template string.
    pub fn interpolate_string(
        template: &str,
        params: &BTreeMap<String, Value>,
        step_outputs: &BTreeMap<String, BTreeMap<String, String>>,
    ) -> String {
        let mut result = template.to_string();

        for (k, v) in params {
            let needle = format!("${{params.{k}}}");
            let rep = match v {
                Value::String(s) => s.clone(),
                other => other.to_string(),
            };
            result = result.replace(&needle, &rep);
        }

        for (step_id, outputs) in step_outputs {
            for (out_key, out_val) in outputs {
                let needle = format!("${{steps.{step_id}.{out_key}}}");
                result = result.replace(&needle, out_val);
            }
        }

        result
    }
}
