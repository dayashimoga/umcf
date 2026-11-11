#!/usr/bin/env bash
# S8.P1.SP2 Five-Tier Test Runner (Linux / macOS / WSL)
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

echo "=== S8.P1.SP2 Chaos Engineering Five-Tier Verification ==="

echo "[1/5] Running [Unit] tests..."
cargo test -p umcf-chaos --test chaos_tests test_asymmetric_network_partition_routing_matrix
cargo test -p umcf-chaos --test chaos_tests test_deterministic_prng_chaos_schedule_reproducibility
cargo test -p umcf-chaos --test chaos_tests test_invariant_monitor_detects_synthetic_split_brain_divergence
echo "  -> [Unit] PASS"

echo "[2/5] Running [Integration] tests..."
cargo test -p umcf-chaos --test chaos_tests test_jepsen_style_majority_minority_split_brain_and_heal_under_load
cargo test -p umcf-chaos --test chaos_tests test_simultaneous_thermal_critical_and_low_battery_live_task_migration
cargo test -p umcf-chaos --test chaos_tests test_mobile_background_freeze_30s_lease_expiry_and_requeue
cargo test -p umcf-chaos --test chaos_tests test_critical_memory_pressure_evicts_cache_and_migrates_worker_without_oom
echo "  -> [Integration] PASS"

echo "[3/5] Running [Simulation] tests..."
cargo test -p umcf-chaos --test chaos_tests test_25_seed_randomized_chaos_campaign_matrix_verifies_all_invariants
echo "  -> [Simulation] PASS"

echo "[4/5] Running [Hardware-Required] tests..."
cargo test -p umcf-chaos --test chaos_tests test_hardware_link_and_thermal_governance_watchdog
echo "  -> [Hardware-Required] PASS"

echo "[5/5] Running [Manually-Verified] tests..."
cargo test -p umcf-chaos --test chaos_tests test_manual_admin_chaos_drawer_schema_and_telemetry
echo "  -> [Manually-Verified] PASS"

echo "=== S8.P1.SP2 Verification: ALL 5 TIERS PASSED (100% PASS RATE) ==="

