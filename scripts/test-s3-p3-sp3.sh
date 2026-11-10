#!/usr/bin/env bash
# scripts/test-s3-p3-sp3.sh
# Multi-tier verification for Subphase S3.P3.SP3
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

echo "=== S3.P3.SP3 Classified Verification Suite ==="

echo "[1/5] Running [Unit] tests for GEMM reference correctness and attestation..."
cargo test -p umcf-hwprobe --test hwprobe_tests test_unit_fp32_gemm_correctness_against_golden_reference
cargo test -p umcf-hwprobe --test hwprobe_tests test_unit_int8_gemm_correctness_against_golden_reference
cargo test -p umcf-hwprobe --test hwprobe_tests test_unit_network_micro_bench_statistics_computation
cargo test -p umcf-hwprobe --test hwprobe_tests test_unit_signed_hardware_attestation_signature_verification

echo "[2/5] Running [Integration] tests for full micro-benchmark and CRDT sync..."
cargo test -p umcf-hwprobe --release --test hwprobe_tests test_integration_full_micro_benchmark_suite_execution -- --nocapture
cargo test -p umcf-hwprobe --test hwprobe_tests test_integration_network_probe_train_async_duplex -- --nocapture
cargo test -p umcf-hwprobe --test hwprobe_tests test_integration_attestation_to_cluster_registry_crdt_sync -- --nocapture

echo "[3/5] Running [Simulation] tests for forged/tampered attestation rejection..."
cargo test -p umcf-hwprobe --test hwprobe_tests test_simulation_forged_or_tampered_attestation_rejection

echo "[4/5] Running [Hardware-Required] tests and Podman quota..."
cargo test -p umcf-hwprobe --test hwprobe_tests test_hardware_required_real_cpu_gemm_and_stream_kernel -- --nocapture
if [ -f "${SCRIPT_DIR}/podman-quota-check.sh" ]; then
    bash "${SCRIPT_DIR}/podman-quota-check.sh"
fi

echo "[5/5] Running [Manually-Verified] signed_hardware_attestation_sample.json audit..."
cargo test -p umcf-hwprobe --test hwprobe_tests test_manually_verified_signed_attestation_sample_export
SAMPLE_PATH="${REPO_ROOT}/artifacts/evidence/stage-03/signed_hardware_attestation_sample.json"
test -f "${SAMPLE_PATH}"

echo "=== All S3.P3.SP3 tests PASSED ==="

