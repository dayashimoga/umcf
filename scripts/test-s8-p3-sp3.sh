#!/usr/bin/env bash
# ==============================================================================
# S8.P3.SP3 Five-Tier Test Execution Script (Linux/macOS)
# Final Hardware Certification Matrix, Release Readiness Audit & v1.0 Sign-Off
# ==============================================================================
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
cd "${REPO_ROOT}"

echo "=== S8.P3.SP3 Five-Tier Verification Suite ==="

# 1. UNIT TESTS
echo "[1/5] Running [Unit] tests..."
cargo test -p release-cert --test release_cert_tests test_certify_v1_gate_scanner_detects_synthetic_todo_macro_or_unchecked_box
cargo test -p release-cert --test release_cert_tests test_documentation_internal_markdown_link_validator_zero_broken_links
echo "  -> [Unit] PASS"

# 2. INTEGRATION TESTS
echo "[2/5] Running [Integration] tests..."
cargo test -p release-cert --test release_cert_tests test_full_10_gate_umcf_certify_v1_execution_passes
cargo test -p release-cert --test release_cert_tests test_clean_install_and_first_run_smoke_test_of_packaged_v1_artifacts
echo "  -> [Integration] PASS"

# 3. SIMULATION TESTS
echo "[3/5] Running [Simulation] tests..."
cargo test -p release-cert --test release_cert_tests test_full_cluster_cold_boot_and_workflow_execution_simulation
echo "  -> [Simulation] PASS"

# 4. HARDWARE-REQUIRED TESTS
echo "[4/5] Running [Hardware-Required] tests..."
cargo test -p release-cert --test release_cert_tests test_final_hardware_certification_matrix_benchmark_verification -- --nocapture
echo "  -> [Hardware-Required] PASS"

# 5. MANUALLY-VERIFIED TESTS
echo "[5/5] Running [Manually-Verified] tests..."
cargo test -p release-cert --test release_cert_tests test_manual_executive_sign_off_receipt_schema
echo "  -> [Manually-Verified] PASS"

echo "=== S8.P3.SP3 All 5 Tiers PASSED (100% Pass Rate) ==="
