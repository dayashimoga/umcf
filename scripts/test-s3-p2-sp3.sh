#!/usr/bin/env bash
# Test runner for Subphase S3.P2.SP3: Roaming, Network Interface Handover & Connection Migration
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

echo "=== Subphase S3.P2.SP3 Test Runner ==="

# 1. Unit tests
echo "[1/5] Running [Unit] tests..."
cargo test -p umcf-net -j 2 --lib -- test_network_interface_watcher

# 2. Integration tests
echo "[2/5] Running [Integration] tests..."
cargo test -p umcf-net -j 2 --release --lib tests::test_quic_connection_migration_under_stream_rebinds -- --nocapture

# 3. Simulation tests
echo "[3/5] Running [Simulation] tests..."
cargo test -p umcf-net -j 2 --lib -- test_abrupt_interface_blackhole_and_rebind_recovery

# 4. Hardware-required tests
echo "[4/5] Running [Hardware-Required] tests..."
cargo test -p umcf-net -j 2 --lib tests::test_hardware_network_interfaces_probe -- --nocapture
bash "${SCRIPT_DIR}/podman-quota-check.sh"

# 5. Manually-verified tests
echo "[5/5] Running [Manually-Verified] tests..."
cargo test -p umcf-net -j 2 --lib -- test_connection_migration_trace_audit

echo "All S3.P2.SP3 tests PASSED successfully."

