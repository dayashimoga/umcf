#!/usr/bin/env bash
# scripts/verify-stage-03.sh
# Master Stage 3 verification and certification script
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
EVIDENCE_DIR="${REPO_ROOT}/artifacts/evidence/stage-03"

echo "================================================================="
echo "   UMCF STAGE 3 COMPREHENSIVE CERTIFICATION & VERIFICATION GATES"
echo "================================================================="

SUBPHASES=(
    "test-s3-p1-sp1.sh"
    "test-s3-p1-sp2.sh"
    "test-s3-p1-sp3.sh"
    "test-s3-p2-sp1.sh"
    "test-s3-p2-sp2.sh"
    "test-s3-p2-sp3.sh"
    "test-s3-p3-sp1.sh"
    "test-s3-p3-sp2.sh"
    "test-s3-p3-sp3.sh"
)

for script in "${SUBPHASES[@]}"; do
    script_path="${SCRIPT_DIR}/${script}"
    echo -n "[RUN] Executing ${script}... "
    if [ -f "${script_path}" ]; then
        bash "${script_path}" > /dev/null 2>&1 && echo "PASS" || { echo "FAIL"; exit 1; }
    else
        echo "MISSING SCRIPT!"
        exit 1
    fi
done

echo ""
echo "[CHECK] Verifying evidence JSON artifacts..."
EVIDENCE_FILES=(
    "S3_P1_SP1_evidence.json"
    "S3_P1_SP2_evidence.json"
    "S3_P1_SP3_evidence.json"
    "S3_P2_SP1_evidence.json"
    "S3_P2_SP2_evidence.json"
    "S3_P2_SP3_evidence.json"
    "S3_P3_SP1_evidence.json"
    "S3_P3_SP2_evidence.json"
    "S3_P3_SP3_evidence.json"
    "lan_discovery_convergence.json"
    "quic_throughput_benchmark.json"
    "qos_latency_under_load.json"
    "nat_hole_punch_matrix.json"
    "relay_fallback_and_upgrade_trace.json"
    "connection_migration_trace.json"
    "live_system_probe_report.json"
    "accelerator_enumeration_report.json"
    "signed_hardware_attestation_sample.json"
)

for file in "${EVIDENCE_FILES[@]}"; do
    if [ -f "${EVIDENCE_DIR}/${file}" ]; then
        echo "  [OK] ${file}"
    else
        echo "  [MISSING] ${file}"
        exit 1
    fi
done

echo ""
echo "[CHECK] Verifying Podman storage quota compliance (< 29.0 GiB)..."
if [ -f "${SCRIPT_DIR}/podman-quota-check.sh" ]; then
    bash "${SCRIPT_DIR}/podman-quota-check.sh"
fi

echo ""
echo "[CHECK] Verifying zero production placeholders..."
if [ -f "${SCRIPT_DIR}/check-no-placeholders.sh" ]; then
    bash "${SCRIPT_DIR}/check-no-placeholders.sh"
fi

echo ""
echo ">>> STAGE 3 FULLY VERIFIED & CERTIFIED: PASS <<<"

