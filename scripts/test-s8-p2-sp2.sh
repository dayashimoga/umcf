#!/usr/bin/env bash
# Subphase S8.P2.SP2 Classified Five-Tier Verification Test Runner
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

echo "========================================================"
echo "  UMCF S8.P2.SP2 Security Red-Team Verification Runner"
echo "========================================================"

# 1. Unit Tests
echo "[1/5] Running [Unit] tests for constant-time comparisons, path traversal blocking, and 0-RTT ticket purging..."
cargo test -p security-redteam --test redteam_tests test_constant_time_api_key_and_pairing_pin_comparison
cargo test -p security-redteam --test redteam_tests test_path_canonicalization_blocks_dotdot_and_symlink_escapes
cargo test -p security-redteam --test redteam_tests test_revoked_certificate_purges_0rtt_session_tickets
echo "  -> [Unit] PASS"

# 2. Integration Tests
echo "[2/5] Running [Integration] tests for 11 red-team vector categories..."
cargo test -p security-redteam --test redteam_tests test_redteam_vectors_1_through_5_sandbox_escape_containment
cargo test -p security-redteam --test redteam_tests test_redteam_vectors_6_through_11_rbac_mtls_and_enclave_containment
echo "  -> [Integration] PASS"

# 3. Simulation Tests
echo "[3/5] Running [Simulation] tests for 45-endpoint 5-privilege RBAC matrix scanner..."
cargo test -p security-redteam --test redteam_tests test_automated_endpoint_scanner_45_methods_5_privilege_tiers_matrix
echo "  -> [Simulation] PASS"

# 4. Hardware-Required Tests
echo "[4/5] Running [Hardware-Required] tests for GPU VRAM zero-scrubbing isolation..."
cargo test -p security-redteam --test redteam_tests test_hardware_vram_zero_scrubbing_isolation_verification
echo "  -> [Hardware-Required] PASS"

# 5. Manually-Verified Tests
echo "[5/5] Running [Manually-Verified] tests for audit ledger and telemetry schema..."
cargo test -p security-redteam --test redteam_tests test_manual_security_self_audit_schema_and_telemetry
echo "  -> [Manually-Verified] PASS"

echo "========================================================"
echo "  Subphase S8.P2.SP2: ALL 5 TIERS PASSED (100% Pass Rate)"
echo "========================================================"

