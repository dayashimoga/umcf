#!/usr/bin/env bash
# Test classification runner for Subphase S2.P3.SP1 (Bash/Linux/macOS)
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

echo "=== Subphase S2.P3.SP1: Local Autonomous Coordinator & SQLite/Redb State Store ==="

echo "[1/5] Running [Unit] tests for HLC and redb commit latency..."
cargo test -p umcf-registry --lib tests::unit_hlc_monotonicity_logical_ticks_and_drift_rejection -- --nocapture
cargo test -p umcf-registry --lib tests::unit_redb_crud_and_transaction_latency_microbenchmark -- --nocapture
cargo test -p umcf-registry --lib tests::unit_crdt_mathematical_laws_10000_randomized_sequences_proptest -- --nocapture
echo "  -> [Unit] PASS"

echo "[2/5] Running [Integration] tests for 3-store mesh all 6 permutation delivery orders..."
cargo test -p umcf-registry --lib tests::integration_three_store_mesh_all_six_permutations_byte_identical_convergence -- --nocapture
echo "  -> [Integration] PASS"

echo "[3/5] Running [Simulation] simulated write crash and ACID recovery test..."
cargo test -p umcf-registry --lib tests::simulation_redb_crash_recovery_acid_consistency -- --nocapture
echo "  -> [Simulation] PASS"

echo "[4/5] Running [Hardware-Required] hardware probe and quota check..."
bash "$SCRIPT_DIR/podman-quota-check.sh"
echo "  -> [Hardware-Required] PASS"

echo "[5/5] Running [Manually-Verified] redb compaction and crdt_proptest_summary.json review..."
cargo test -p umcf-registry --lib tests::manually_verified_redb_compaction_and_file_size_growth_metrics -- --nocapture
if [ -f "$REPO_ROOT/artifacts/evidence/stage-02/crdt_proptest_summary.json" ] && \
   grep -q "commutativity" "$REPO_ROOT/artifacts/evidence/stage-02/crdt_proptest_summary.json" && \
   grep -q "idempotency" "$REPO_ROOT/artifacts/evidence/stage-02/crdt_proptest_summary.json"; then
  echo "  -> [Manually-Verified] PASS"
else
  echo "ERROR: invalid or missing crdt_proptest_summary.json"
  exit 1
fi

echo "=== Subphase S2.P3.SP1: ALL TEST TIERS PASSED ==="
