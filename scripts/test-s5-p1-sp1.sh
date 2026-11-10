#!/usr/bin/env bash
# Test verification script for Subphase S5.P1.SP1 across all 5 test tiers.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

echo "=== S5.P1.SP1 Five-Tier Test Verification Suite ==="

echo "[1/5] Running [Unit] tests..."
cargo test -p umcf-gguf --test store_and_sync_tests test_unit_

echo "[2/5] Running [Integration] tests..."
cargo test -p umcf-gguf --test store_and_sync_tests test_integration_ -- --nocapture
cargo test -p umcf-daemon integration_daemon_model_registry_service -- --nocapture

echo "[3/5] Running [Simulation] tests..."
cargo test -p umcf-gguf --test store_and_sync_tests test_simulation_ -- --nocapture

echo "[4/5] Running [Hardware-Required] tests..."
cargo test -p umcf-gguf --test store_and_sync_tests test_hardware_ -- --nocapture
bash "${REPO_ROOT}/scripts/enforce-podman-quota.sh" || true

echo "[5/5] Running [Manually-Verified] tests..."
cargo test -p umcf-gguf --test store_and_sync_tests test_manual_

echo "=== S5.P1.SP1 Verification: ALL PASS ==="

