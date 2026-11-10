#!/usr/bin/env bash
# ==============================================================================
# scripts/verify-stage-01.sh — UMCF Stage 1 Master Verification & Certification
# Subphase S1.P3.SP3
# ==============================================================================
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
cd "${REPO_ROOT}"

python3 "${SCRIPT_DIR}/verify_stage_01.py" || python "${SCRIPT_DIR}/verify_stage_01.py"
