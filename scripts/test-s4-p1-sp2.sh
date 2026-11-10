#!/usr/bin/env bash
# scripts/test-s4-p1-sp2.sh
# Multi-tier verification for Subphase S4.P1.SP2
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

echo "=== S4.P1.SP2 Classified Verification Suite ==="

echo "[1/5] Running [Unit] tests for ThermalBatteryGuard transitions and hysteresis..."
cargo test -p umcf-guard --test guard_tests test_unit_thermal_battery_guard_regime_transitions
cargo test -p umcf-guard --test guard_tests test_unit_hysteresis_deadband_oscillation_prevention
cargo test -p umcf-guard --test guard_tests test_unit_battery_hysteresis_deadband

echo "[2/5] Running [Integration] tests for sensor trajectory stream and OS onTrimMemory..."
cargo test -p umcf-guard --test guard_tests test_integration_sensor_stream_throttle_pause_resume -- --nocapture
cargo test -p umcf-guard --test guard_tests test_integration_os_trim_memory_emergency_evict -- --nocapture

echo "[3/5] Running [Simulation] tests for abrupt battery drop and unplug fault injection..."
cargo test -p umcf-guard --test guard_tests test_simulation_abrupt_battery_drop_and_unplug_fault_injection

echo "[4/5] Running [Hardware-Required] tests and Podman quota..."
cargo test -p umcf-guard --test guard_tests test_hw_required_guard_real_clock_and_podman_quota -- --nocapture
if [ -f "${SCRIPT_DIR}/podman-quota-check.sh" ]; then
    bash "${SCRIPT_DIR}/podman-quota-check.sh"
fi

echo "[5/5] Running [Manually-Verified] thermal_battery_hysteresis_trace.json audit..."
cargo test -p umcf-guard --test guard_tests test_manually_verified_thermal_battery_hysteresis_trace_audit
TRACE_PATH="${REPO_ROOT}/artifacts/evidence/stage-04/thermal_battery_hysteresis_trace.json"
test -f "${TRACE_PATH}"

echo "=== All S4.P1.SP2 tests PASSED ==="
