#!/usr/bin/env bash
# scripts/test-s7-p1-sp2.sh
# Executes classified five-tier verification tests for Subphase S7.P1.SP2

set -euo pipefail

echo "=== Running Subphase S7.P1.SP2 Classified Five-Tier Verification Suite ==="

echo "[1/5] Running [Unit] tests..."
cargo test -p umcf-storage --test erasure_placement_tests test_convergent_xchacha20poly1305_
cargo test -p umcf-storage --test erasure_placement_tests test_rs_4_2_and_6_3_all_combinations_
cargo test -p umcf-storage --test erasure_placement_tests test_corrupted_shard_detected_
cargo test -p umcf-storage --test erasure_placement_tests test_failure_domain_placement_

echo "[2/5] Running [Integration] tests..."
cargo test -p umcf-storage --test erasure_placement_tests test_6_node_rs_4_2_upload_kill_2_nodes_download_and_self_heal_

echo "[3/5] Running [Simulation] tests..."
cargo test -p umcf-storage --test erasure_placement_tests test_20_node_cluster_churn_simulation_

echo "[4/5] Running [Hardware-Required] tests..."
cargo test -p umcf-storage --test erasure_placement_tests test_hardware_simd_reed_solomon_ -- --nocapture

echo "[5/5] Running [Manually-Verified] tests..."
cargo test -p umcf-storage --test erasure_placement_tests test_manual_shard_health_matrix_

echo "=== Subphase S7.P1.SP2: 100% PASS Across All 5 Tiers ==="

