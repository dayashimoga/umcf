#!/usr/bin/env bash
# Test runner for Subphase S2.P3.SP3: Google Cloud Run Fallback & Split-Brain Reconciliation
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

echo "=== Subphase S2.P3.SP3 Test Suite ==="

echo "[1/4] Running Cloud Run fallback unit & integration tests..."
cargo test -p umcf-cloudrun-fallback --bin umcf-cloudrun-fallback

echo "[2/4] Running circuit breaker, failover client, and split-brain reconciliation tests..."
cargo test -p umcf-registry --lib tests::unit_failover_circuit_breaker_state_transitions
cargo test -p umcf-registry --lib tests::integration_failover_client_switches_to_cloudrun_and_buffers_for_lan
cargo test -p umcf-registry --lib tests::simulation_split_brain_partition_healing_and_state_convergence

echo "[3/4] Running Podman storage quota check (< 29.0 GiB)..."
if [ -f "${SCRIPT_DIR}/podman-quota-check.sh" ]; then
  bash "${SCRIPT_DIR}/podman-quota-check.sh"
fi

echo "[4/4] Verifying evidence reports..."
if [ -f "${REPO_ROOT}/artifacts/evidence/stage-02/split_brain_reconciliation_trace.json" ]; then
  echo "Evidence trace verified: artifacts/evidence/stage-02/split_brain_reconciliation_trace.json exists."
else
  echo "ERROR: Reconciliation trace missing!"
  exit 1
fi

echo "All S2.P3.SP3 tests PASSED successfully."
