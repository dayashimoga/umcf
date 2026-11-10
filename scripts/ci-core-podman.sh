#!/usr/bin/env bash
# ==============================================================================
# scripts/ci-core-podman.sh — UMCF Rust Core CI Pipeline Runner
# Subphase S1.P3.SP1
# ==============================================================================
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
cd "${REPO_ROOT}"

echo "=============================================================================="
echo " UMCF Core CI Pipeline (Rust Workspace & Quality Gates)"
echo "=============================================================================="

# Step 1: Pre-step Quota & Placeholder Guard
echo "[Step 1/7] Running pre-build storage quota and placeholder check..."
bash "${SCRIPT_DIR}/check-no-placeholders.sh"
bash "${SCRIPT_DIR}/podman-quota-check.sh"

# Step 2: Code Formatting
echo "[Step 2/7] Checking code formatting with cargo fmt..."
cargo fmt --all -- --check

# Step 3: Clippy Static Analysis (Zero Warnings)
echo "[Step 3/7] Running static analysis with cargo clippy..."
cargo clippy --workspace --all-targets -- -D warnings

# Step 4: Workspace Test Suite
echo "[Step 4/7] Running workspace test suite..."
if command -v cargo-nextest >/dev/null 2>&1; then
  cargo nextest run --workspace
else
  cargo test --workspace
fi

# Step 5: Coverage Generation
echo "[Step 5/7] Generating workspace coverage report..."
mkdir -p "${REPO_ROOT}/artifacts/evidence/stage-01"
python3 "${SCRIPT_DIR}/generate-stage01-coverage.py"

# Step 6: Coverage Threshold Verification (>= 85.0%)
echo "[Step 6/7] Enforcing coverage threshold gate (>= 85.0%)..."
python3 "${SCRIPT_DIR}/verify-coverage-threshold.py" \
  --report "${REPO_ROOT}/artifacts/evidence/stage-01/coverage_stage01.json" \
  --threshold 85.0

# Step 7: Post-step Quota Guard
echo "[Step 7/7] Running post-build storage quota verification..."
bash "${SCRIPT_DIR}/podman-quota-check.sh"

echo "=============================================================================="
echo " UMCF Core CI Pipeline: ALL QUALITY GATES PASSED (0 Warnings, >=85% Coverage)"
echo "=============================================================================="
