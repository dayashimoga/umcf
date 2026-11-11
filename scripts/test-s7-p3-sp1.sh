#!/usr/bin/env bash
# ==============================================================================
# Subphase S7.P3.SP1 Five-Tier Test Runner: Signed Cryptographic Resource Receipts
# ==============================================================================
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

echo "=== Subphase S7.P3.SP1 Classified Verification ==="

echo "[1/5] Running [Unit] tests..."
cargo test -p umcf-ledger --test ledger_tests test_canonical_receipt_serialization_and_dual_ed25519_signatures
cargo test -p umcf-ledger --test ledger_tests test_receipt_hash_chain_detects_missing_or_reordered_sequence
cargo test -p umcf-ledger --test ledger_tests test_sanity_bounds_checker_rejects_inflated_token_or_pixel_claims
echo "  -> [Unit] PASS"

echo "[2/5] Running [Integration] tests..."
cargo test -p umcf-ledger --test ledger_tests test_end_to_end_wasm_ai_gpu_media_tasks_produce_countersigned_receipts
cargo test -p umcf-ledger --test ledger_tests test_streaming_session_emits_60s_incremental_checkpoint_receipts
echo "  -> [Integration] PASS"

echo "[3/5] Running [Simulation] tests..."
cargo test -p umcf-ledger --test ledger_tests test_malicious_worker_replay_and_inflation_rejection
echo "  -> [Simulation] PASS"

echo "[4/5] Running [Hardware-Required] tests..."
cargo test -p umcf-ledger --test ledger_tests test_ed25519_batch_signature_verification_throughput
echo "  -> [Hardware-Required] PASS"

echo "[5/5] Running [Manually-Verified] tests..."
cargo test -p umcf-ledger --test ledger_tests test_manual_receipt_explorer_schema_and_dual_signature_modal
echo "  -> [Manually-Verified] PASS"

echo "========================================================"
echo " Subphase S7.P3.SP1 Verification: ALL 5 TIERS PASSED"
echo "========================================================"
