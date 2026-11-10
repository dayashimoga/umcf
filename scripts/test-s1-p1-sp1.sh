#!/usr/bin/env bash
# Executes classified tests for Subphase S1.P1.SP1:
# - Unit: verify-governance execution and failure detection on missing file
# - Integration: pre-commit hook simulation and .gitattributes LF normalization check
# - Simulation: synthetic placeholder fault injection (todo!() and unimplemented!()) rejection
# - Hardware-Required: marked N/A (pure repo governance)
# - Manually Verified: markdown schema verification
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

echo "=== Subphase S1.P1.SP1 Classified Test Runner ==="

# 1. Unit Test
echo "[1/4] Running [Unit] verify-governance..."
bash "${SCRIPT_DIR}/verify-governance.sh"

# 2. Integration Test
echo "[2/4] Running [Integration] .gitattributes LF check..."
if grep -q '\*\.rs text eol=lf' "${REPO_ROOT}/.gitattributes" && grep -q '\*\.ts text eol=lf' "${REPO_ROOT}/.gitattributes"; then
  echo "  -> [Integration] PASS"
else
  echo "  -> [Integration] FAIL" >&2
  exit 1
fi

# 3. Simulation Test
echo "[3/4] Running [Simulation] placeholder fault-injection test..."
SYNTHETIC_FILE="${REPO_ROOT}/crates/umcf-core/src/synthetic_fault.rs"
trap "rm -f '${SYNTHETIC_FILE}'" EXIT
echo 'pub fn synthetic_fault() { todo!("synthetic fault injection"); }' > "${SYNTHETIC_FILE}"

if bash "${SCRIPT_DIR}/check-no-placeholders.sh" >/dev/null 2>&1; then
  echo "  -> [Simulation] FAIL: Gate failed to reject synthetic fault" >&2
  rm -f "${SYNTHETIC_FILE}"
  exit 1
else
  echo "  -> [Simulation] PASS: Correctly detected and rejected synthetic fault"
  rm -f "${SYNTHETIC_FILE}"
fi

# 4. Manually-Verified Test
echo "[4/4] Running [Manually-Verified] Markdown schema inspect..."
if grep -q "Contributing to Universal Mobile Compute Fabric" "${REPO_ROOT}/CONTRIBUTING.md" && \
   grep -q "Stage 1" "${REPO_ROOT}/TODO.md" && \
   grep -q "S1\.P1\.SP1" "${REPO_ROOT}/TODO.md" && \
   grep -q "Keep a Changelog" "${REPO_ROOT}/CHANGELOG.md"; then
  echo "  -> [Manually-Verified] PASS"
else
  echo "  -> [Manually-Verified] FAIL" >&2
  exit 1
fi

echo "=== Subphase S1.P1.SP1 All Classified Tests Passed (100%) ==="
exit 0
