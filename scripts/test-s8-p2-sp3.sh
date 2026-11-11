#!/usr/bin/env bash
# Subphase S8.P2.SP3 Classified Five-Tier Verification Test Runner
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

echo "========================================================"
echo "  UMCF S8.P2.SP3 Supply Chain Compliance Runner"
echo "========================================================"

# 1. Unit Tests
echo "[1/5] Running [Unit] tests for SLSA v1.0 provenance, tampering rejection, and SPDX license expressions..."
cargo test -p supply-chain --test supply_chain_tests test_slsa_provenance_and_sbom_ed25519_signature_verification
cargo test -p supply-chain --test supply_chain_tests test_tampered_binary_or_sbom_hash_rejected_by_provenance_verifier
cargo test -p supply-chain --test supply_chain_tests test_spdx_license_expression_parser_rejects_gpl_and_agpl
echo "  -> [Unit] PASS"

# 2. Integration Tests
echo "[2/5] Running [Integration] tests for license compliance, bit-exact reproducible builds, and path leak checks..."
cargo test -p supply-chain --test supply_chain_tests test_cargo_deny_and_polyglot_dependency_license_compliance_gate
cargo test -p supply-chain --test supply_chain_tests test_dual_directory_reproducible_build_bit_exact_binary_comparison
cargo test -p supply-chain --test supply_chain_tests test_binary_contains_zero_host_workspace_path_strings
echo "  -> [Integration] PASS"

# 3. Simulation Tests
echo "[3/5] Running [Simulation] tests for StrictVerifiedBuildOnly cluster admission..."
cargo test -p supply-chain --test supply_chain_tests test_peer_rejected_in_strict_verified_build_only_cluster
echo "  -> [Simulation] PASS"

# 4. Hardware-Required Tests
echo "[4/5] Running [Hardware-Required] tests for release SBOM generation and Ed25519 signing..."
cargo test -p supply-chain --test supply_chain_tests test_full_release_sbom_generation_and_signature_verification
echo "  -> [Hardware-Required] PASS"

# 5. Manually-Verified Tests
echo "[5/5] Running [Manually-Verified] tests for umcf version --verify-provenance CLI and viewer schema..."
cargo test -p supply-chain --test supply_chain_tests test_manual_sbom_and_licenses_viewer_schema
echo "  -> [Manually-Verified] PASS"

echo "========================================================"
echo "  Subphase S8.P2.SP3: ALL 5 TIERS PASSED (100% Pass Rate)"
echo "========================================================"

