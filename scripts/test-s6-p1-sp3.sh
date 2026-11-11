#!/usr/bin/env bash
set -euo pipefail

echo "=== S6.P1.SP3 Five-Tier Test Runner ==="
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

echo "[1/5] Running [Unit] tests..."
cargo test -p umcf-gpu --test verifier_tests test_unit_

echo "[2/5] Running [Integration] tests..."
cargo test -p umcf-gpu --test verifier_tests test_integration_

echo "[3/5] Running [Simulation] tests..."
cargo test -p umcf-gpu --test verifier_tests test_simulation_

echo "[4/5] Running [Hardware-Required] tests..."
cargo test -p umcf-gpu --test verifier_tests test_hardware_

echo "[5/5] Running [Manually-Verified] tests..."
cargo test -p umcf-gpu --test verifier_tests test_manual_

mkdir -p artifacts/stage-06/p1 artifacts/evidence/stage-06/p1
cp -r artifacts/stage-06/p1/* artifacts/evidence/stage-06/p1/ 2>/dev/null || true

echo "=== S6.P1.SP3 Verification Complete: ALL PASS ==="

