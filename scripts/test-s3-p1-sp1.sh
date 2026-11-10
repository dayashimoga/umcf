#!/usr/bin/env bash
# Subphase S3.P1.SP1 Test Runner
# Verifies Unit, Integration, Simulation, Hardware-Required, and Manually-Verified tiers for S3.P1.SP1

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

echo "=== Subphase S3.P1.SP1 Test Suite ==="

# 1. Unit Tests
echo "[1/5] Running [Unit] tests..."
cargo test -p umcf-net -j 2 --lib -- test_signed test_token test_replay test_tamper test_invalid

# 2. Integration Tests
echo "[2/5] Running [Integration] tests..."
cargo test -p umcf-net -j 2 --lib tests::test_three_node_loopback_discovery_convergence -- --nocapture

# 3. Simulation Tests
echo "[3/5] Running [Simulation] tests..."
cargo test -p umcf-net -j 2 --lib -- test_foreign_and_malformed test_beacon_flood

# 4. Hardware-Required Tests
echo "[4/5] Running [Hardware-Required] tests..."
cargo test -p umcf-net -j 2 --lib tests::test_hardware_network_interfaces_probe -- --nocapture
bash "${SCRIPT_DIR}/podman-quota-check.sh"

# 5. Manually-Verified Tests
echo "[5/5] Checking convergence manifest..."
if [ ! -f "${REPO_ROOT}/artifacts/evidence/stage-03/lan_discovery_convergence.json" ]; then
  echo "Error: lan_discovery_convergence.json missing!"
  exit 1
fi

echo "=== Subphase S3.P1.SP1 All Tests Passed Successfully ==="
