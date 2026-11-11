#!/usr/bin/env bash
# S8.P1.SP1 Five-Tier Test Runner (Linux / macOS / WSL)
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

echo "=== S8.P1.SP1 Multi-OS Cluster Integration Five-Tier Verification ==="

echo "[1/5] Running [Unit] tests..."
cargo test -p e2e-cluster --test multi_os_tests test_virtual_path_normalizer_rejects_windows_backslashes_and_drive_letters_on_wire
cargo test -p e2e-cluster --test multi_os_tests test_case_collision_detector_for_cross_os_storage_manifests
cargo test -p e2e-cluster --test multi_os_tests test_16kb_page_size_mmap_alignment_for_android15_and_apple_silicon
echo "  -> [Unit] PASS"

echo "[2/5] Running [Integration] tests..."
cargo test -p e2e-cluster --test multi_os_tests test_scenario_1_to_6_full_multi_os_concurrent_cluster_suite
echo "  -> [Integration] PASS"

echo "[3/5] Running [Simulation] tests..."
cargo test -p e2e-cluster --test multi_os_tests test_simulated_heterogeneous_os_clock_and_page_size_drift
echo "  -> [Simulation] PASS"

echo "[4/5] Running [Hardware-Required] tests..."
cargo test -p e2e-cluster --test multi_os_tests test_hardware_host_os_interop_telemetry_receipt
echo "  -> [Hardware-Required] PASS"

echo "[5/5] Running [Manually-Verified] tests..."
cargo test -p e2e-cluster --test multi_os_tests test_manual_multi_os_cluster_topology_ui_inspection
echo "  -> [Manually-Verified] PASS"

echo "=== S8.P1.SP1 Verification: ALL 5 TIERS PASSED (100% PASS RATE) ==="

