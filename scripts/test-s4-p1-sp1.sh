#!/usr/bin/env bash
# scripts/test-s4-p1-sp1.sh
# Multi-tier verification for Subphase S4.P1.SP1
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

echo "=== S4.P1.SP1 Classified Verification Suite ==="

echo "[1/5] Running [Unit] tests for VramBudgetTracker, Wasm limiter, and cgroup v2 formatting..."
cargo test -p umcf-guard --test guard_tests test_unit_vram_budget_tracker_concurrent_reservations
cargo test -p umcf-guard --test guard_tests test_unit_vram_budget_tracker_raii_lease_drop_release
cargo test -p umcf-guard --test guard_tests test_unit_wasm_resource_limiter_pages
cargo test -p umcf-guard --test guard_tests test_unit_cgroup_v2_control_file_formatting

echo "[2/5] Running [Integration] tests for CPU/memory workload enforcement and policy updates..."
cargo test -p umcf-guard --test guard_tests test_integration_cpu_memory_intensive_workload_enforcement -- --nocapture
cargo test -p umcf-guard --test guard_tests test_integration_cpu_utilization_window_measurement -- --nocapture

echo "[3/5] Running [Simulation] tests for runaway allocation loop rejection..."
cargo test -p umcf-guard --test guard_tests test_simulation_runaway_allocation_loop_rejection

echo "[4/5] Running [Hardware-Required] tests and Podman quota..."
cargo test -p umcf-guard --test guard_tests test_hw_required_cgroup_v2_or_job_object_limits -- --nocapture
if [ -f "${SCRIPT_DIR}/podman-quota-check.sh" ]; then
    bash "${SCRIPT_DIR}/podman-quota-check.sh"
fi

echo "[5/5] Running [Manually-Verified] quota_enforcement_report.json audit..."
cargo test -p umcf-guard --test guard_tests test_manually_verified_quota_enforcement_report_audit
REPORT_PATH="${REPO_ROOT}/artifacts/evidence/stage-04/quota_enforcement_report.json"
test -f "${REPORT_PATH}"

echo "=== All S4.P1.SP1 tests PASSED ==="
