#!/usr/bin/env bash
# Test Runner for Subphase S7.P2.SP3: Declarative Workload Manifests & Reproducible Job Templates
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

echo "=== [1/5] [Unit] YAML/TOML schema, Kahn DAG cycles, interpolation, lockfile ==="
cargo test -p umcf-manifest --test manifest_tests test_manifest_yaml_and_toml_parser_deny_unknown_fields
cargo test -p umcf-manifest --test manifest_tests test_dag_cycle_detection_and_topological_stage_grouping
cargo test -p umcf-manifest --test manifest_tests test_parameter_and_step_output_interpolation_type_checker
cargo test -p umcf-manifest --test manifest_tests test_lockfile_blake3_mismatch_rejected_under_frozen_lockfile

echo "=== [2/5] [Integration] All 6 built-in templates and hybrid pipeline ==="
cargo test -p umcf-manifest --test manifest_tests test_all_6_builtin_job_templates_init_lock_and_execute_e2e
cargo test -p umcf-manifest --test manifest_tests test_multi_step_hybrid_wasm_gpu_ai_dag_pipeline_execution

echo "=== [3/5] [Simulation] Workflow failure resumption and cached output reuse ==="
cargo test -p umcf-manifest --test manifest_tests test_workflow_resumption_reuses_cached_step_outputs_without_reexecution

echo "=== [4/5] [Hardware-Required] Real hardware execution timings ==="
cargo test -p umcf-manifest --test manifest_tests test_hardware_execution_timings_hybrid_media_ai_pipeline

echo "=== [5/5] [Manually-Verified] Template Gallery UI schema and DAG states ==="
cargo test -p umcf-manifest --test manifest_tests test_manual_template_gallery_and_dag_schema_conformance

echo "All 5 tiers for S7.P2.SP3 PASSED successfully!"
