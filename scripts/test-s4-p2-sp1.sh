#!/usr/bin/env bash
set -euo pipefail

# Subphase S4.P2.SP1 test runner
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${REPO_ROOT}"

echo "[1/5] Running [Unit] tests for JobDag, constraint pruning, and Roofline cost model..."
cargo test -p umcf-scheduler --test scheduler_tests test_s4_p2_sp1_unit_dag_topological_sort_and_cycle_detection
cargo test -p umcf-scheduler --test scheduler_tests test_s4_p2_sp1_unit_constraint_solver_pruning_matrix
cargo test -p umcf-scheduler --test scheduler_tests test_s4_p2_sp1_unit_roofline_cost_model_lan_vs_wan_tradeoff
echo "  -> [Unit] PASS"

echo "[2/5] Running [Integration] 100-task DAG scheduled across 32 nodes strictly < 10 ms..."
cargo test -p umcf-scheduler --test scheduler_tests test_s4_p2_sp1_integration_100_task_dag_across_32_nodes_under_10ms -- --nocapture
echo "  -> [Integration] PASS"

echo "[3/5] Running [Simulation] 1,000 randomized DAGs and cluster states..."
cargo test -p umcf-scheduler --test scheduler_tests test_s4_p2_sp1_simulation_randomized_1000_dags_proptest_style -- --nocapture
echo "  -> [Simulation] PASS"

echo "[4/5] Running [Hardware-Required] monotonic timer and Podman quota..."
cargo test -p umcf-scheduler --test scheduler_tests test_s4_p2_sp1_hardware_monotonic_timer_and_quota
bash scripts/podman-quota-check.sh
echo "  -> [Hardware-Required] PASS"

echo "[5/5] Running [Manually-Verified] scheduler_placement_explain.json audit..."
cargo test -p umcf-scheduler --test scheduler_tests test_s4_p2_sp1_manually_verified_explain_json_audit
test -f artifacts/evidence/stage-04/scheduler_placement_explain.json
echo "  -> [Manually-Verified] PASS"

echo "=== Subphase S4.P2.SP1 PASS ==="

