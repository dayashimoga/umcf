#!/usr/bin/env bash
# ==============================================================================
# scripts/verify-coverage-threshold.sh — UMCF Coverage Threshold Verifier
# ==============================================================================
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PYTHON_BIN="$(which python3 || which python || true)"

if [[ -n "${PYTHON_BIN}" ]]; then
  exec "${PYTHON_BIN}" "${SCRIPT_DIR}/verify-coverage-threshold.py" "$@"
else
  echo "ERROR: python3 or python required for coverage verification"
  exit 1
fi
