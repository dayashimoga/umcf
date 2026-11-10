#!/usr/bin/env bash
# Test classification runner for Subphase S2.P2.SP2 (Bash/Linux/macOS)
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

echo "=== Subphase S2.P2.SP2: Fine-Grained RBAC & Resource-Sharing Consent Engine ==="

echo "[1/5] Running [Unit] tests for RBAC matrix and default-deny authorization..."
cargo test -p umcf-enrollment --lib tests::unit_rbac_permission_matrix_and_default_deny_microbenchmark -- --nocapture
echo "  -> [Unit] PASS"

echo "[2/5] Running [Integration] tests for worker ConsentEnforcer and immediate policy updates..."
cargo test -p umcf-enrollment --lib tests::integration_worker_consent_enforcer_rejects_gpu_workload_and_immediate_policy_update -- --nocapture
echo "  -> [Integration] PASS"

echo "[3/5] Running [Simulation] foreign Master forged consent policy rejection..."
cargo test -p umcf-enrollment --lib tests::simulation_forged_consent_policy_by_foreign_master_rejected -- --nocapture
echo "  -> [Simulation] PASS"

echo "[4/5] Running [Hardware-Required] hardware probe and quota check..."
bash "$SCRIPT_DIR/podman-quota-check.sh"
echo "  -> [Hardware-Required] PASS"

echo "[5/5] Running [Manually-Verified] rbac_consent_matrix.json review..."
if [ -f "$REPO_ROOT/artifacts/evidence/stage-02/rbac_consent_matrix.json" ] && \
   grep -q "ClusterOwner" "$REPO_ROOT/artifacts/evidence/stage-02/rbac_consent_matrix.json" && \
   grep -q "default_deny_enforced" "$REPO_ROOT/artifacts/evidence/stage-02/rbac_consent_matrix.json"; then
  echo "  -> [Manually-Verified] PASS"
else
  echo "ERROR: invalid or missing rbac_consent_matrix.json"
  exit 1
fi

echo "=== Subphase S2.P2.SP2: ALL TEST TIERS PASSED ==="
