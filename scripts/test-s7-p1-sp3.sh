#!/usr/bin/env bash
# ==============================================================================
# test-s7-p1-sp3.sh — S7.P1.SP3 Five-Tier Verification Test Suite
# Hierarchical Pinning, Offline Caching, W-TinyLFU, and Concurrent GC
# ==============================================================================
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

echo "========================================================"
echo "  UMCF Subphase S7.P1.SP3 Five-Tier Verification"
echo "========================================================"

# Tier 1: Unit Tests
echo "[1/5] Running [Unit] tests..."
cargo test -p umcf-storage --test pinning_gc_tests test_pin_recursive_reference_counting_shared_dedup_chunks
cargo test -p umcf-storage --test pinning_gc_tests test_w_tinylfu_admission_filter_protects_hot_chunks_against_one_hit_wonders
cargo test -p umcf-storage --test pinning_gc_tests test_lease_with_ttl_expiration_makes_chunks_eligible_for_gc
cargo test -p umcf-storage --test pinning_gc_tests test_gc_grace_window_protects_in_flight_uncommitted_chunks
echo "  -> [Unit] PASS"

# Tier 2: Integration Tests
echo "[2/5] Running [Integration] tests..."
cargo test -p umcf-storage --test pinning_gc_tests test_pin_hydrate_from_erasure_shards_disconnect_network_and_read_offline
cargo test -p umcf-storage --test pinning_gc_tests test_concurrent_ingest_pin_unpin_and_gc_sweep_zero_dangling_refs
cargo test -p umcf-storage --test pinning_gc_tests test_idle_bitrot_scrubber_detects_corrupted_local_chunk_and_repairs_from_peer
echo "  -> [Integration] PASS"

# Tier 3: Simulation Tests
echo "[3/5] Running [Simulation] tests..."
cargo test -p umcf-storage --test pinning_gc_tests test_1000_object_creations_partial_overlapping_ttl_leases_and_50_gc_cycles
echo "  -> [Simulation] PASS"

# Tier 4: Hardware-Required Tests
echo "[4/5] Running [Hardware-Required] tests..."
cargo test -p umcf-storage --test pinning_gc_tests test_hardware_gc_mark_and_sweep_latency_benchmark_10000_chunks -- --nocapture
echo "  -> [Hardware-Required] PASS"

# Tier 5: Manually-Verified Tests
echo "[5/5] Running [Manually-Verified] tests..."
cargo test -p umcf-storage --test pinning_gc_tests test_manual_offline_pin_manager_ui_schema
echo "  -> [Manually-Verified] PASS"

echo "========================================================"
echo "  S7.P1.SP3 Verification Complete: 5/5 TIERS PASS"
echo "========================================================"
