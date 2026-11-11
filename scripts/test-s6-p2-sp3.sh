#!/usr/bin/env bash
# S6.P2.SP3 Five-Tier Classified Verification Script
# Media Pipeline Fault Recovery & Quality Verification

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

echo "=== Running S6.P2.SP3 Five-Tier Classified Verification Suite ==="

echo "[1/5] Running [Unit] tests for structural frame count/IDR validation, 3-anchor SSIM/PSNR, and watchdog requeue..."
cargo test -p umcf-media --test quality_and_recovery_tests test_structural_
cargo test -p umcf-media --test quality_and_recovery_tests test_3_anchor_
cargo test -p umcf-media --test quality_and_recovery_tests test_stalled_
echo "  -> [Unit] PASS"

echo "[2/5] Running [Integration] tests for corrupted segment fallback re-transcode and crash checkpoint resume..."
cargo test -p umcf-media --test quality_and_recovery_tests test_corrupted_
cargo test -p umcf-media --test quality_and_recovery_tests test_coordinator_crash_
echo "  -> [Integration] PASS"

echo "[3/5] Running [Simulation] tests for chaos transcode resilience (25% kills, 10% corruption)..."
cargo test -p umcf-media --test quality_and_recovery_tests test_simulation_chaos_
echo "  -> [Simulation] PASS"

echo "[4/5] Running [Hardware-Required] tests for 3-anchor quality verification overhead (<30ms)..."
cargo test -p umcf-media --test quality_and_recovery_tests test_hardware_
echo "  -> [Hardware-Required] PASS"

echo "[5/5] Running [Manually-Verified] tests for segment health grid and quality telemetry UI schema..."
cargo test -p umcf-media --test quality_and_recovery_tests test_manual_
echo "  -> [Manually-Verified] PASS"

STAGE_DIR="${REPO_ROOT}/artifacts/stage-06/p2"
EVIDENCE_DIR="${REPO_ROOT}/artifacts/evidence/stage-06/p2"
mkdir -p "${STAGE_DIR}" "${EVIDENCE_DIR}"

if command -v pwsh &> /dev/null; then
  pwsh "${SCRIPT_DIR}/test-s6-p2-sp3.ps1"
else
  echo "Artifacts generated in test suite run."
fi

echo "=== S6.P2.SP3 All 5 Tiers Passed (100% Pass Rate) ==="

