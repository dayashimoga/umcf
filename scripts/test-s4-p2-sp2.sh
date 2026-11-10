#!/usr/bin/env bash
set -euo pipefail

# Subphase S4.P2.SP2 test runner
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${REPO_ROOT}"

echo "[1/5] Running [Unit] tests for water-filling partitioner and data locality..."
cargo test -p umcf-scheduler --test scheduler_tests test_s4_p2_sp2_unit_water_filling_partition_ratio
echo "  -> [Unit] PASS"

echo "[2/5] Running [Integration] heterogeneous speedup over single desktop and uniform 1/3..."
cargo test -p umcf-scheduler --test scheduler_tests test_s4_p2_sp2_integration_speedup_over_desktop_and_uniform -- --nocapture
echo "  -> [Integration] PASS"

echo "[3/5] Running [Simulation] work stealing straggler mitigation and WAN pruning..."
cargo test -p umcf-scheduler --test scheduler_tests test_s4_p2_sp2_simulation_work_stealing_straggler_mitigation -- --nocapture
cargo test -p umcf-scheduler --test scheduler_tests test_s4_p2_sp2_simulation_wan_transfer_overhead_pruning -- --nocapture
echo "  -> [Simulation] PASS"

echo "[4/5] Running [Hardware-Required] monotonic timer and Podman quota..."
cargo test -p umcf-scheduler --test scheduler_tests test_s4_p2_sp2_hardware_monotonic_timer_and_quota
bash scripts/podman-quota-check.sh
echo "  -> [Hardware-Required] PASS"

echo "[5/5] Running [Manually-Verified] heterogeneous_partition_speedup.json audit..."
cargo test -p umcf-scheduler --test scheduler_tests test_s4_p2_sp2_manually_verified_speedup_evidence_export
test -f artifacts/evidence/stage-04/heterogeneous_partition_speedup.json
echo "  -> [Manually-Verified] PASS"

echo "=== Subphase S4.P2.SP2 PASS ==="

