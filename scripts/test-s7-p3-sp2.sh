#!/usr/bin/env bash
# ==============================================================================
# Subphase S7.P3.SP2 Five-Tier Test Runner: Double-Entry Ledger & Marketplace Matcher
# ==============================================================================
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

echo "=== Subphase S7.P3.SP2 Classified Verification ==="

echo "[1/5] Running [Unit] tests..."
cargo test -p umcf-marketplace --test marketplace_tests test_double_entry_journal_rejects_unbalanced_legs_and_negative_balances
cargo test -p umcf-marketplace --test marketplace_tests test_escrow_lock_partial_receipt_settlement_and_unused_refund_exact_math
cargo test -p umcf-marketplace --test marketplace_tests test_idempotent_receipt_settlement_prevents_double_payout
cargo test -p umcf-marketplace --test marketplace_tests test_byzantine_verification_failure_evidence_slashes_provider_bond
cargo test -p umcf-marketplace --test marketplace_tests test_marketplace_matcher_respects_charging_wifi_and_privacy_constraints
echo "  -> [Unit] PASS"

echo "[2/5] Running [Integration] tests..."
cargo test -p umcf-marketplace --test marketplace_tests test_full_marketplace_lifecycle_ask_bid_match_escrow_execute_receipt_settle_refund
cargo test -p umcf-marketplace --test marketplace_tests test_concurrent_64_thread_settlements_preserve_global_zero_sum_invariant
echo "  -> [Integration] PASS"

echo "[3/5] Running [Simulation] tests..."
cargo test -p umcf-marketplace --test marketplace_tests test_proptest_randomized_market_bids_asks_and_slashings_conserve_zero_sum
echo "  -> [Simulation] PASS"

echo "[4/5] Running [Hardware-Required] tests..."
cargo test -p umcf-marketplace --test marketplace_tests test_marketplace_matching_engine_throughput_benchmark
echo "  -> [Hardware-Required] PASS"

echo "[5/5] Running [Manually-Verified] tests..."
cargo test -p umcf-marketplace --test marketplace_tests test_manual_contribute_device_and_order_book_ui_schema
echo "  -> [Manually-Verified] PASS"

echo "========================================================"
echo " Subphase S7.P3.SP2 Verification: ALL 5 TIERS PASSED"
echo "========================================================"
