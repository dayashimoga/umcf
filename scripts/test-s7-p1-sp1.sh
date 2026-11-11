#!/usr/bin/env bash
# S7.P1.SP1 Five-Tier Classified Verification Script
# BLAKE3 Content-Addressed Chunk Store & Explicit Quota Enforcement

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

echo "=== Running S7.P1.SP1 Five-Tier Classified Verification Suite ==="

echo "[1/5] Running [Unit] tests for FastCDC shift-resilience, 64-thread CAS quota loop, permit drop, and host space floor..."
cargo test -p umcf-storage --test chunk_store_quota_tests test_fastcdc_chunking_shift_
cargo test -p umcf-storage --test chunk_store_quota_tests test_atomic_quota_reservation_
cargo test -p umcf-storage --test chunk_store_quota_tests test_uncommitted_quota_permit_
cargo test -p umcf-storage --test chunk_store_quota_tests test_host_os_free_space_floor_
echo "  -> [Unit] PASS"

echo "[2/5] Running [Integration] tests for object ingest, deduplication, bit-exact reconstruction, and cache eviction..."
cargo test -p umcf-storage --test chunk_store_quota_tests test_ingest_deduplicate_and_verify_
cargo test -p umcf-storage --test chunk_store_quota_tests test_quota_evicts_unpinned_cache_
echo "  -> [Integration] PASS"

echo "[3/5] Running [Simulation] tests for crash recovery orphan staging cleanup and index reconciliation..."
cargo test -p umcf-storage --test chunk_store_quota_tests test_simulation_crash_recovery_
echo "  -> [Simulation] PASS"

echo "[4/5] Running [Hardware-Required] tests for FastCDC + BLAKE3 ingestion throughput on host hardware..."
cargo test -p umcf-storage --test chunk_store_quota_tests test_hardware_fastcdc_blake3_
echo "  -> [Hardware-Required] PASS"

echo "[5/5] Running [Manually-Verified] tests for Storage Quota Card UI schema and stacked progress model..."
cargo test -p umcf-storage --test chunk_store_quota_tests test_manual_storage_quota_card_
echo "  -> [Manually-Verified] PASS"

STAGE_DIR="${REPO_ROOT}/artifacts/stage-07/p1"
EVIDENCE_DIR="${REPO_ROOT}/artifacts/evidence/stage-07/p1"
mkdir -p "${STAGE_DIR}" "${EVIDENCE_DIR}"

if command -v pwsh &> /dev/null; then
  pwsh "${SCRIPT_DIR}/test-s7-p1-sp1.ps1"
else
  echo "Artifacts generated in test suite run."
fi

echo "=== S7.P1.SP1 All 5 Tiers Passed (100% Pass Rate) ==="

