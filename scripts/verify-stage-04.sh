#!/usr/bin/env bash
# Stage 4 Master Verification & Certification Script (UMCF)
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

echo "=== Stage 4 Master Verification & Certification ==="

# 1. Run all 9 subphase test runners
bash "${SCRIPT_DIR}/test-s4-p1-sp1.sh"
bash "${SCRIPT_DIR}/test-s4-p1-sp2.sh"
bash "${SCRIPT_DIR}/test-s4-p1-sp3.sh"
bash "${SCRIPT_DIR}/test-s4-p2-sp1.sh"
bash "${SCRIPT_DIR}/test-s4-p2-sp2.sh"
bash "${SCRIPT_DIR}/test-s4-p2-sp3.sh"
bash "${SCRIPT_DIR}/test-s4-p3-sp1.sh"
bash "${SCRIPT_DIR}/test-s4-p3-sp2.sh"
bash "${SCRIPT_DIR}/test-s4-p3-sp3.sh"

# 2. Check Podman storage quota compliance (< 29.0 GiB)
bash "${SCRIPT_DIR}/podman-quota-check.sh"

# 3. Check zero placeholders
bash "${SCRIPT_DIR}/check-no-placeholders.sh"

# 4. Confirm Stage 4 certification artifact exists
test -f "${REPO_ROOT}/artifacts/evidence/stage-04/STAGE_04_CERTIFICATION.json"

echo "=== STAGE 4 FULLY VERIFIED & CERTIFIED: PASS ==="

