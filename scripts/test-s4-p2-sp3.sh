#!/usr/bin/env bash
set -euo pipefail

# Subphase S4.P2.SP3 test runner
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${REPO_ROOT}"

echo "[1/5] Running [Unit] tests for DRR fairness, priority aging, and admission control..."
cargo test -p umcf-scheduler --test scheduler_tests test_s4_p2_sp3_unit_drr_fairness_aging_and_admission_rejection
echo "  -> [Unit] PASS"

echo "[2/5] Running [Integration] InteractiveRealtime bypass of 50 batch jobs < 15 ms..."
cargo test -p umcf-scheduler --test scheduler_tests test_s4_p2_sp3_integration_interactive_bypass_under_15ms -- --nocapture
echo "  -> [Integration] PASS"

echo "[3/5] Running [Simulation] deadlock-free gang scheduling (1,000 runs) and worker backpressure..."
cargo test -p umcf-scheduler --test scheduler_tests test_s4_p2_sp3_simulation_deadlock_free_gang_scheduling -- --nocapture
cargo test -p umcf-scheduler --test scheduler_tests test_s4_p2_sp3_simulation_worker_backpressure_saturation -- --nocapture
echo "  -> [Simulation] PASS"

echo "[4/5] Running [Hardware-Required] monotonic timer and Podman quota..."
cargo test -p umcf-scheduler --test scheduler_tests test_s4_p2_sp3_hardware_monotonic_timer_and_quota
bash scripts/podman-quota-check.sh
echo "  -> [Hardware-Required] PASS"

echo "[5/5] Running [Manually-Verified] queue_fairness_and_backpressure.json audit..."
cargo test -p umcf-scheduler --test scheduler_tests test_s4_p2_sp3_manually_verified_queue_fairness_json_audit
test -f artifacts/evidence/stage-04/queue_fairness_and_backpressure.json
echo "  -> [Manually-Verified] PASS"

echo "=== Subphase S4.P2.SP3 PASS ==="

