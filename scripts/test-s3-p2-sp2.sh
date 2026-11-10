#!/usr/bin/env bash
# Test runner for Subphase S3.P2.SP2: Encrypted TURN/DERP-Style Relay Fallback Protocol
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

echo "=== Subphase S3.P2.SP2 Test Runner ==="

# 1. Unit tests
echo "[1/5] Running [Unit] tests..."
cargo test -p umcf-net -j 2 --lib -- test_relay_frame_binary_codec test_relay_server_routing

# 2. Integration tests
echo "[2/5] Running [Integration] tests..."
cargo test -p umcf-net -j 2 --release --lib tests::test_relay_fallback_connection_and_automatic_path_upgrade -- --nocapture

# 3. Simulation tests
echo "[3/5] Running [Simulation] tests..."
cargo test -p umcf-net -j 2 --lib -- test_malicious_relay_payload_tampering

# 4. Hardware-required tests
echo "[4/5] Running [Hardware-Required] tests..."
cargo test -p umcf-net -j 2 --lib tests::test_hardware_network_interfaces_probe -- --nocapture
bash "${SCRIPT_DIR}/podman-quota-check.sh"

# 5. Manually-verified tests
echo "[5/5] Running [Manually-Verified] tests..."
cargo test -p umcf-net -j 2 --lib -- test_relay_fallback_and_upgrade_trace_audit

echo "All S3.P2.SP2 tests PASSED successfully."

