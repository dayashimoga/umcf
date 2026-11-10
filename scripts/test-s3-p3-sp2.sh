#!/usr/bin/env bash
# Test script for Subphase S3.P3.SP2 (GPU & NPU Compute Enumeration)
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

echo "=== [1/5] Running [Unit] tests for PCI vendor classification and unified memory heuristics ==="
cargo test -p umcf-hwprobe --test hwprobe_tests test_unit_pci_vendor_classification
cargo test -p umcf-hwprobe --test hwprobe_tests test_unit_unified_memory_heuristics

echo "=== [2/5] Running [Integration] AcceleratorProber::enumerate() live query (< 150 ms) ==="
cargo test -p umcf-hwprobe --release --test hwprobe_tests test_integration_accelerator_enumeration_live -- --nocapture

echo "=== [3/5] Running [Simulation] graceful CPU SIMD fallback ==="
cargo test -p umcf-hwprobe --test hwprobe_tests test_simulation_cuda_rocm_absence_and_graceful_cpu_fallback

echo "=== [4/5] Running [Hardware-Required] physical GPU/NPU probe and Podman quota ==="
cargo test -p umcf-hwprobe --test hwprobe_tests test_hw_required_accelerator_enumeration -- --nocapture
bash "$SCRIPT_DIR/podman-quota-check.sh"

echo "=== [5/5] Running [Manually-Verified] accelerator_enumeration_report.json audit ==="
cargo test -p umcf-hwprobe --test hwprobe_tests test_manually_verified_accelerator_enumeration_report_audit

REPORT_PATH="$REPO_ROOT/artifacts/evidence/stage-03/accelerator_enumeration_report.json"
if [ ! -f "$REPORT_PATH" ]; then
  echo "Error: accelerator_enumeration_report.json not found at $REPORT_PATH"
  exit 1
fi

echo "=== Subphase S3.P3.SP2 All Classified Tests Passed Successfully ==="
exit 0
