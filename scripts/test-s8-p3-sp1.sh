#!/usr/bin/env bash
# ==============================================================================
# S8.P3.SP1 Five-Tier Test Execution Script (Linux/macOS)
# Data-Plane Zero-Copy Optimization & Mobile Battery Impact Tuning
# ==============================================================================
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
cd "${REPO_ROOT}"

echo "=== S8.P3.SP1 Five-Tier Verification Suite ==="

# 1. UNIT TESTS
echo "[1/5] Running [Unit] tests..."
cargo test -p data-plane --test data_plane_tests test_aligned_slab_buffer_pool_64b_and_16kb_alignment_and_auto_return_on_drop
cargo test -p data-plane --test data_plane_tests test_mmap_to_bytes_slice_preserves_exact_pointer_address_zero_alloc
cargo test -p data-plane --test data_plane_tests test_timer_coalescer_batches_heartbeats_into_single_radio_wakeup_window
cargo test -p data-plane --test data_plane_tests test_power_governor_state_transitions_ac_to_eco_to_deep_saver
echo "  -> [Unit] PASS"

# 2. INTEGRATION TESTS
echo "[2/5] Running [Integration] tests..."
cargo test -p data-plane --test data_plane_tests test_zero_copy_quic_4mib_shard_stream_zero_payload_heap_copies
cargo test -p data-plane --test data_plane_tests test_battery_deep_saver_gracefully_drains_and_rejects_remote_worker_tasks
echo "  -> [Integration] PASS"

# 3. SIMULATION TESTS
echo "[3/5] Running [Simulation] tests..."
cargo test -p data-plane --test data_plane_tests test_24_hour_mobile_battery_model_simulation_wakeups
echo "  -> [Simulation] PASS"

# 4. HARDWARE-REQUIRED TESTS
echo "[4/5] Running [Hardware-Required] tests..."
cargo test -p data-plane --test data_plane_tests test_data_plane_zero_copy_throughput_and_benchmarks -- --nocapture
echo "  -> [Hardware-Required] PASS"

# 5. MANUALLY-VERIFIED TESTS
echo "[5/5] Running [Manually-Verified] tests..."
cargo test -p data-plane --test data_plane_tests test_manual_mobile_power_profile_and_wakeup_schema
echo "  -> [Manually-Verified] PASS"

echo "=== S8.P3.SP1 All 5 Tiers PASSED (100% Pass Rate) ==="
