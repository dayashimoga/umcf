#!/usr/bin/env bash
# Test classification runner for Subphase S2.P2.SP3 (Bash/Linux/macOS)
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

echo "=== Subphase S2.P2.SP3: Dynamic Master/Worker/Hybrid State Machine & Election ==="

echo "[1/5] Running [Unit] tests for MasterFitnessScore ranking and tie-breaking..."
cargo test -p umcf-enrollment --lib tests::unit_master_fitness_score_ranking_across_heterogeneous_profiles -- --nocapture
echo "  -> [Unit] PASS"

echo "[2/5] Running [Integration] tests for 5-node cluster election failover & leader transfer..."
cargo test -p umcf-enrollment --lib tests::integration_five_node_cluster_election_failover_and_leader_transfer -- --nocapture
if [ ! -f "$REPO_ROOT/artifacts/evidence/stage-02/election_failover_trace.json" ]; then
  echo "ERROR: election_failover_trace.json not found!"
  exit 1
fi
echo "  -> [Integration] PASS"

echo "[3/5] Running [Simulation] 7-node simultaneous election timeouts under 15% packet loss..."
cargo test -p umcf-enrollment --lib tests::simulation_simultaneous_election_timeouts_under_packet_loss -- --nocapture
echo "  -> [Simulation] PASS"

echo "[4/5] Running [Hardware-Required] hardware probe and quota check..."
bash "$SCRIPT_DIR/podman-quota-check.sh"
echo "  -> [Hardware-Required] PASS"

echo "[5/5] Running [Manually-Verified] election_failover_trace.json review..."
if [ -f "$REPO_ROOT/artifacts/evidence/stage-02/election_failover_trace.json" ] && \
   grep -q "initial_leader" "$REPO_ROOT/artifacts/evidence/stage-02/election_failover_trace.json" && \
   grep -q "split_brain_count" "$REPO_ROOT/artifacts/evidence/stage-02/election_failover_trace.json"; then
  echo "  -> [Manually-Verified] PASS"
else
  echo "ERROR: invalid or missing election_failover_trace.json"
  exit 1
fi

echo "=== Subphase S2.P2.SP3: ALL TEST TIERS PASSED ==="
