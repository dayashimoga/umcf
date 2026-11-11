#!/usr/bin/env bash
# S8.P1.SP3 Five-Tier Test Runner (Linux / macOS / WSL)
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

echo "=== S8.P1.SP3 Soak Testing & Zero-Leak Profiling Five-Tier Verification ==="

echo "[1/5] Running [Unit] tests..."
cargo test -p soak-suite --test soak_tests test_ols_linear_regression_leak_slope_detector_catches_16_byte_per_cycle_leak
cargo test -p soak-suite --test soak_tests test_fd_and_os_handle_counter_accuracy_windows_linux_macos
cargo test -p soak-suite --test soak_tests test_quiescence_barrier_waits_for_background_drop_tasks
echo "  -> [Unit] PASS"

echo "[2/5] Running [Integration] tests..."
cargo test -p soak-suite --test soak_tests test_1000_cycle_ci_accelerated_multi_workload_soak_zero_leak_gate
cargo test -p soak-suite --test soak_tests test_abrupt_client_disconnect_storm_zero_socket_or_stream_handle_leak
echo "  -> [Integration] PASS"

echo "[3/5] Running [Simulation] tests..."
cargo test -p soak-suite --test soak_tests test_10000_cycle_accelerated_soak_with_churn_and_cancellations
echo "  -> [Simulation] PASS"

echo "[4/5] Running [Hardware-Required] tests..."
cargo test -p soak-suite --test soak_tests test_hardware_process_rss_and_handle_leak_stability
echo "  -> [Hardware-Required] PASS"

echo "[5/5] Running [Manually-Verified] tests..."
cargo test -p soak-suite --test soak_tests test_manual_resource_and_leak_diagnostics_ui_schema
echo "  -> [Manually-Verified] PASS"

echo "=== S8.P1.SP3 Verification: ALL 5 TIERS PASSED (100% PASS RATE) ==="
