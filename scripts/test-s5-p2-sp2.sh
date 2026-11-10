#!/usr/bin/env bash
# Test script for Subphase S5.P2.SP2: Hardware Backend Selection with Fallback
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

echo "=== Running S5.P2.SP2 Test Harness ==="
cargo test -p umcf-ai --test backend_selector_tests

mkdir -p "${REPO_ROOT}/artifacts/stage-05/p2"
mkdir -p "${REPO_ROOT}/artifacts/evidence/stage-05/p2"

echo "S5.P2.SP2 tests passed successfully."
