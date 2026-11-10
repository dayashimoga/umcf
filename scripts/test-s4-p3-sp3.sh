#!/usr/bin/env bash
# Test runner for Subphase S4.P3.SP3: End-to-End Distributed Job Execution Verification
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

echo "=== Subphase S4.P3.SP3 Verification Runner ==="

# 1. Unit Tests
echo "[1/5] Running [Unit] tests..."
cargo test -p umcf-worker --test worker_tests test_unit_s4_p3_sp3_

# 2. Integration Tests
echo "[2/5] Running [Integration] tests..."
cargo test -p umcf-worker --test worker_tests test_integration_s4_p3_sp3_ -- --nocapture

# 3. Simulation Tests
echo "[3/5] Running [Simulation] tests..."
cargo test -p umcf-worker --test worker_tests test_simulation_s4_p3_sp3_ -- --nocapture

# 4. Hardware-Required Tests
echo "[4/5] Running [Hardware-Required] tests..."
cargo test -p umcf-worker --test worker_tests test_hardware_s4_p3_sp3_
bash "${REPO_ROOT}/scripts/podman-quota-check.sh"

# 5. Manually-Verified Tests
echo "[5/5] Running [Manually-Verified] tests..."
cargo test -p umcf-worker --test worker_tests test_manually_verified_s4_p3_sp3_
test -f "${REPO_ROOT}/artifacts/evidence/stage-04/e2e_distributed_dag_report.json"

echo "=== Subphase S4.P3.SP3: ALL TIERS PASSED ==="

