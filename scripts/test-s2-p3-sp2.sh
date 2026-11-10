#!/usr/bin/env bash
# Test runner for Subphase S2.P3.SP2: Cloudflare Workers/Durable Objects Control Plane Sync
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

echo "=== Subphase S2.P3.SP2 Test Suite ==="

echo "[1/4] Running Cloudflare Worker test suite (Unit, Integration, Simulation)..."
node --test "${REPO_ROOT}/services/cf_control_plane/test/worker.test.mjs"

echo "[2/4] Running Rust CloudflareControlPlaneClient unit & integration tests..."
cargo test -p umcf-registry --lib tests::unit_cloudflare_client_sign_headers_format_and_crypto
cargo test -p umcf-registry --lib tests::unit_cloudflare_client_url_construction_and_validation
cargo test -p umcf-registry --lib tests::integration_rust_client_against_local_control_plane_server

echo "[3/4] Running Podman storage quota check (< 29.0 GiB)..."
if [ -f "${SCRIPT_DIR}/podman-quota-check.sh" ]; then
  bash "${SCRIPT_DIR}/podman-quota-check.sh"
fi

echo "[4/4] Verifying evidence report..."
if [ -f "${REPO_ROOT}/artifacts/evidence/stage-02/cf_worker_vitest_report.json" ]; then
  echo "Evidence report verified: artifacts/evidence/stage-02/cf_worker_vitest_report.json exists."
else
  echo "ERROR: Evidence report missing!"
  exit 1
fi

echo "All S2.P3.SP2 tests PASSED successfully."
