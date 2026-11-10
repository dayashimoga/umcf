#!/usr/bin/env bash
set -euo pipefail

# test-s3-p1-sp2.sh - S3.P1.SP2 5-tier test classification runner
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

echo "=== Running Subphase S3.P1.SP2 Tests ==="

echo "[1/5] Running [Unit] tests..."
cargo test -p umcf-net -j 2 --lib -- test_quic_stream_type test_ed25519_pkcs8 test_x509_cert_info

echo "[2/5] Running [Integration] tests..."
cargo test -p umcf-net -j 2 --release --lib tests::test_quic_mtls_handshake_and_mutual_attestation -- --nocapture

echo "[3/5] Running [Simulation] tests..."
cargo test -p umcf-net -j 2 --lib -- test_quic_mtls_revoked_cert test_quic_mtls_foreign_cluster

echo "[4/5] Running [Hardware-Required] tests..."
cargo test -p umcf-net -j 2 --lib tests::test_hardware_network_interfaces_probe -- --nocapture
bash "${SCRIPT_DIR}/podman-quota-check.sh"

echo "[5/5] Running [Manually-Verified] tests..."
cargo test -p umcf-net -j 2 --lib tests::test_quic_throughput_benchmark_manifest_audit -- --nocapture

echo "=== All Subphase S3.P1.SP2 Tests PASSED ==="

