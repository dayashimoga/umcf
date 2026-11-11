#!/usr/bin/env bash
set -euo pipefail

# S5.P2.SP3 Five-Tier Verification Script (Linux/macOS)
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

echo "=== Running S5.P2.SP3 Classified Verification Suite ==="

echo "[1/5] Running [Unit] tests..."
cargo test -p umcf-ai --test streaming_and_kv_tests test_unit_

echo "[2/5] Running [Integration] tests..."
cargo test -p umcf-ai --test streaming_and_kv_tests test_integration_

echo "[3/5] Running [Simulation] tests..."
cargo test -p umcf-ai --test streaming_and_kv_tests test_simulation_

echo "[4/5] Running [Hardware-Required] tests..."
cargo test -p umcf-ai --test streaming_and_kv_tests test_hardware_

echo "[5/5] Running [Manually-Verified] tests..."
cargo test -p umcf-ai --test streaming_and_kv_tests test_manual_

mkdir -p "$REPO_ROOT/artifacts/stage-05/p2" "$REPO_ROOT/artifacts/evidence/stage-05/p2"

echo "=== S5.P2.SP3 Verification Complete: ALL PASS ==="
