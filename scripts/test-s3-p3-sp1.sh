#!/usr/bin/env bash
# Test script for Subphase S3.P3.SP1 (Cross-Platform CPU/RAM/Storage/Battery/Thermal Probing)
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

echo "=== [1/5] Running [Unit] tests for CPU/RAM/Battery/Thermal parsers ==="
cargo test -p umcf-hwprobe --test hwprobe_tests test_unit_

echo "=== [2/5] Running [Integration] SystemProber::probe_all() live query (< 25 ms) ==="
cargo test -p umcf-hwprobe --release --test hwprobe_tests test_integration_system_probe_live -- --nocapture

echo "=== [3/5] Running [Simulation] thermal critical, low battery, and cgroup clamping ==="
cargo test -p umcf-hwprobe --test hwprobe_tests test_simulation_

echo "=== [4/5] Running [Hardware-Required] CPUID SIMD flags and Podman quota ==="
cargo test -p umcf-hwprobe --test hwprobe_tests test_hw_required_ -- --nocapture
bash "$SCRIPT_DIR/podman-quota-check.sh"

echo "=== [5/5] Running [Manually-Verified] live_system_probe_report.json audit ==="
cargo test -p umcf-hwprobe --test hwprobe_tests test_manually_verified_

REPORT_PATH="$REPO_ROOT/artifacts/evidence/stage-03/live_system_probe_report.json"
if [ ! -f "$REPORT_PATH" ]; then
  echo "Error: live_system_probe_report.json not found at $REPORT_PATH"
  exit 1
fi

echo "=== Subphase S3.P3.SP1 All Classified Tests Passed Successfully ==="
exit 0
