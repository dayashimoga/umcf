#!/usr/bin/env bash
set -euo pipefail

# Subphase S4.P1.SP3 test runner
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${REPO_ROOT}"

echo "[1/5] Running [Unit] tests for UserActivityDetector and InstantPreemptor..."
cargo test -p umcf-guard --test guard_tests test_s4_p1_sp3_unit_activity_state_machine_and_rampup
cargo test -p umcf-guard --test guard_tests test_s4_p1_sp3_unit_cooldown_and_rampup_interruption
cargo test -p umcf-guard --test guard_tests test_s4_p1_sp3_unit_preemption_token_and_cooperative_yield
echo "  -> [Unit] PASS"

echo "[2/5] Running [Integration] multi-worker preemption latency strictly < 50 ms..."
cargo test -p umcf-guard --test guard_tests test_s4_p1_sp3_integration_worker_preemption_latency_under_50ms -- --nocapture
echo "  -> [Integration] PASS"

echo "[3/5] Running [Simulation] 100 Hz bursts and host CPU load spike preemption..."
cargo test -p umcf-guard --test guard_tests test_s4_p1_sp3_simulation_rapid_100hz_bursts -- --nocapture
cargo test -p umcf-guard --test guard_tests test_s4_p1_sp3_simulation_host_cpu_spike_detection -- --nocapture
echo "  -> [Simulation] PASS"

echo "[4/5] Running [Hardware-Required] monotonic system clock and Podman quota..."
cargo test -p umcf-guard --test guard_tests test_s4_p1_sp3_hardware_monotonic_timer_and_quota
bash scripts/podman-quota-check.sh
echo "  -> [Hardware-Required] PASS"

echo "[5/5] Running [Manually-Verified] preemption_latency_histogram.json audit..."
cargo test -p umcf-guard --test guard_tests test_s4_p1_sp3_manually_verified_latency_histogram_export
test -f artifacts/evidence/stage-04/preemption_latency_histogram.json
echo "  -> [Manually-Verified] PASS"

echo "=== Subphase S4.P1.SP3 PASS ==="
