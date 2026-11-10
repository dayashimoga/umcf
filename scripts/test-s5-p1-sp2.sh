#!/usr/bin/env bash
# Test verification script for Subphase S5.P1.SP2 across all 5 test tiers.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

echo "=== S5.P1.SP2 Five-Tier Test Verification Suite ==="

echo "[1/5] Running [Unit] tests..."
cargo test -p umcf-gguf --test parser_and_estimator_tests test_unit_

echo "[2/5] Running [Integration] tests..."
cargo test -p umcf-gguf --test parser_and_estimator_tests test_integration_ -- --nocapture

echo "[3/5] Running [Simulation] tests..."
cargo test -p umcf-gguf --test parser_and_estimator_tests test_simulation_ -- --nocapture

echo "[4/5] Running [Hardware-Required] tests..."
cargo test -p umcf-gguf --test parser_and_estimator_tests test_hardware_ -- --nocapture
bash "${REPO_ROOT}/scripts/enforce-podman-quota.sh" || true

echo "[5/5] Running [Manually-Verified] tests..."
cargo test -p umcf-gguf --test parser_and_estimator_tests test_manual_

echo "=== S5.P1.SP2 Verification: ALL PASS ==="
