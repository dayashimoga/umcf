#!/usr/bin/env bash
# Test runner for Subphase S3.P2.SP1: STUN/ICE Candidate Gathering & UDP Hole Punching
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

echo "=== Subphase S3.P2.SP1 Test Runner ==="

# 1. Unit tests
echo "[1/5] Running [Unit] tests..."
cargo test -p umcf-net -j 2 --lib -- test_rfc8489_stun test_ice_candidate_priority test_hole_punch_probe test_nat_type_punchability

# 2. Integration tests
echo "[2/5] Running [Integration] tests..."
cargo test -p umcf-net -j 2 --release --lib tests::test_nat_port_restricted_cone_hole_punching_and_quic_mtls -- --nocapture

# 3. Simulation tests
echo "[3/5] Running [Simulation] tests..."
cargo test -p umcf-net -j 2 --lib -- test_symmetric_nat_clean_rejection_requires_relay

# 4. Hardware-required tests
echo "[4/5] Running [Hardware-Required] tests..."
cargo test -p umcf-net -j 2 --lib tests::test_hardware_network_interfaces_probe -- --nocapture
bash "${SCRIPT_DIR}/podman-quota-check.sh"

# 5. Manually-verified tests
echo "[5/5] Running [Manually-Verified] tests..."
cargo test -p umcf-net -j 2 --lib -- test_nat_hole_punch_matrix_audit

echo "All S3.P2.SP1 tests PASSED successfully."

