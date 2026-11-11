#!/usr/bin/env bash
# S8.P2.SP1 Five-Tier Test Runner (Linux / macOS / WSL)
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

echo "=== S8.P2.SP1 Protocol Parser & FFI Fuzzing Five-Tier Verification ==="

echo "[1/5] Running [Unit] tests..."
cargo test -p fuzz-suite --test fuzz_tests test_length_prefix_allocation_bomb_2_pow_62_rejected_with_zero_heap_alloc
cargo test -p fuzz-suite --test fuzz_tests test_deeply_nested_gguf_array_and_json_schema_recursion_limit_enforced
cargo test -p fuzz-suite --test fuzz_tests test_minimized_fuzz_corpus_regression_all_15_targets
echo "  -> [Unit] PASS"

echo "[2/5] Running [Integration] tests..."
cargo test -p fuzz-suite --test fuzz_tests test_proptest_fuzz_wire_and_tokens
echo "  -> [Integration] PASS"

echo "[3/5] Running [Simulation] tests..."
cargo test -p fuzz-suite --test fuzz_tests test_live_socket_simulated_fuzzer_malformed_quic_and_http
echo "  -> [Simulation] PASS"

echo "[4/5] Running [Hardware-Required] tests..."
cargo test -p fuzz-suite --test fuzz_tests test_hardware_multicore_parallel_fuzz_stability
echo "  -> [Hardware-Required] PASS"

echo "[5/5] Running [Manually-Verified] tests..."
cargo test -p fuzz-suite --test fuzz_tests test_manual_fuzz_summary_report_and_coverage_schema
echo "  -> [Manually-Verified] PASS"

echo "=== S8.P2.SP1 Verification: ALL 5 TIERS PASSED (100% PASS RATE) ==="

