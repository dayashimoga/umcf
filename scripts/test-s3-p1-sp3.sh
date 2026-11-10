#!/usr/bin/env bash
set -euo pipefail

# test-s3-p1-sp3.sh - S3.P1.SP3 5-tier test classification runner
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

echo "=== Running Subphase S3.P1.SP3 Tests ==="

echo "[1/5] Running [Unit] tests..."
cargo test -p umcf-net -j 2 --lib -- test_traffic_class_priorities test_bandwidth_shaper_token_accounting

echo "[2/5] Running [Integration] tests..."
cargo test -p umcf-net -j 2 --release --lib tests::test_qos_under_bulk_load_latency_and_zero_drops -- --nocapture

echo "[3/5] Running [Simulation] tests..."
cargo test -p umcf-net -j 2 --lib -- test_bandwidth_shaper_dynamic_throttling_convergence

echo "[4/5] Running [Hardware-Required] tests..."
cargo test -p umcf-net -j 2 --lib tests::test_hardware_network_interfaces_probe -- --nocapture
bash "${SCRIPT_DIR}/podman-quota-check.sh"

echo "[5/5] Running [Manually-Verified] tests..."
cargo test -p umcf-net -j 2 --lib -- test_qos_openmetrics test_qos_latency_under_load_manifest_audit -- --nocapture

echo "=== All Subphase S3.P1.SP3 Tests PASSED ==="
