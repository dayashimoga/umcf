#!/usr/bin/env bash
# Test runner for Subphase S4.P3.SP1: Isolated Worker Execution Sandbox & Artifact Staging
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

echo "=== Subphase S4.P3.SP1 Verification Runner ==="

# 1. Unit Tests
echo "[1/5] Running [Unit] tests..."
cargo test -p umcf-worker --test worker_tests test_unit_

# 2. Integration Tests
echo "[2/5] Running [Integration] tests..."
cargo test -p umcf-worker --test worker_tests test_integration_ -- --nocapture

# 3. Simulation Tests
echo "[3/5] Running [Simulation] tests..."
cargo test -p umcf-worker --test worker_tests test_simulation_ -- --nocapture

# 4. Hardware-Required Tests
echo "[4/5] Running [Hardware-Required] tests..."
cargo test -p umcf-worker --test worker_tests test_hardware_
bash "${REPO_ROOT}/scripts/podman-quota-check.sh"

# 5. Manually-Verified Tests
echo "[5/5] Running [Manually-Verified] tests..."
cargo test -p umcf-worker --test worker_tests test_manually_verified_
test -f "${REPO_ROOT}/artifacts/evidence/stage-04/sandbox_escape_audit.json"

echo "=== Subphase S4.P3.SP1: ALL TIERS PASSED ==="
