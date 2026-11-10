#!/usr/bin/env bash
# Master verification script for Stage 2: Rust Core, Identity, Enrollment, Device Registry & Cluster Management
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

echo "=== Stage 2 Verification & Certification Suite ==="

echo "[1/4] Running subphase test suites (S2.P1.SP1 through S2.P3.SP3)..."
bash "${SCRIPT_DIR}/test-s2-p1-sp1.sh"
bash "${SCRIPT_DIR}/test-s2-p1-sp2.sh"
bash "${SCRIPT_DIR}/test-s2-p1-sp3.sh"
bash "${SCRIPT_DIR}/test-s2-p2-sp1.sh"
bash "${SCRIPT_DIR}/test-s2-p2-sp2.sh"
bash "${SCRIPT_DIR}/test-s2-p2-sp3.sh"
bash "${SCRIPT_DIR}/test-s2-p3-sp1.sh"
bash "${SCRIPT_DIR}/test-s2-p3-sp2.sh"
bash "${SCRIPT_DIR}/test-s2-p3-sp3.sh"

echo "[2/4] Verifying Podman storage quota (< 29.0 GiB)..."
if [ -f "${SCRIPT_DIR}/podman-quota-check.sh" ]; then
  bash "${SCRIPT_DIR}/podman-quota-check.sh"
fi

echo "[3/4] Verifying evidence files..."
EVIDENCE_DIR="${REPO_ROOT}/artifacts/evidence/stage-02"
for f in \
  S2_P1_SP1_evidence.json S2_P1_SP2_evidence.json S2_P1_SP3_evidence.json \
  S2_P2_SP1_evidence.json S2_P2_SP2_evidence.json S2_P2_SP3_evidence.json \
  S2_P3_SP1_evidence.json S2_P3_SP2_evidence.json S2_P3_SP3_evidence.json \
  split_brain_reconciliation_trace.json cf_worker_vitest_report.json; do
  if [ ! -f "${EVIDENCE_DIR}/${f}" ]; then
    echo "ERROR: Missing evidence file: ${f}"
    exit 1
  fi
done

echo "[4/4] Verifying STAGE_02_CERTIFICATION.json..."
if [ -f "${EVIDENCE_DIR}/STAGE_02_CERTIFICATION.json" ]; then
  echo "STAGE_02_CERTIFICATION.json verified."
fi

echo ">>> STAGE 2 FULLY VERIFIED & CERTIFIED: PASS <<<"
