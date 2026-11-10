#!/usr/bin/env bash
# ==============================================================================
# scripts/ci-web-podman.sh — UMCF Web & Edge Control Plane CI Pipeline Runner
# Subphase S1.P3.SP1
# ==============================================================================
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
cd "${REPO_ROOT}"

START_TIME=$(date +%s)

echo "=============================================================================="
echo " UMCF Web & Edge Control Plane CI Pipeline"
echo "=============================================================================="

# Step 1: Pre-step Quota & Placeholder Guard
echo "[Step 1/5] Running pre-build storage quota and placeholder check..."
bash "${SCRIPT_DIR}/check-no-placeholders.sh"
bash "${SCRIPT_DIR}/podman-quota-check.sh"

# Step 2: Web Console Lint & Unit Tests
echo "[Step 2/5] Running @umcf/web_console lint and unit tests..."
node ./apps/web_console/scripts/check.mjs
node --test ./apps/web_console/test/console.test.mjs

# Step 3: Cloudflare Control Plane Unit Tests
echo "[Step 3/5] Running @umcf/cf_control_plane unit tests..."
node --test ./services/cf_control_plane/test/worker.test.mjs

# Step 4: Web & Admin Dashboard Lint & Unit Tests
echo "[Step 4/5] Running @umcf/web-dashboard lint and unit tests..."
node ./apps/web-dashboard/scripts/check.mjs
node --test ./apps/web-dashboard/test/dashboard.test.mjs

# Step 5: Post-step Quota Guard
echo "[Step 5/5] Running post-build storage quota verification..."
bash "${SCRIPT_DIR}/podman-quota-check.sh"

END_TIME=$(date +%s)
DURATION=$((END_TIME - START_TIME))

echo "=============================================================================="
echo " UMCF Web CI Pipeline: ALL CHECKS PASSED in ${DURATION}s (< 60s target)"
echo "=============================================================================="
