#!/usr/bin/env bash
# Test classification runner for Subphase S2.P2.SP1 (Bash/Linux/macOS)
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

echo "=== Subphase S2.P2.SP1: QR Enrollment Token Generator & Scanner Flow ==="

echo "[1/5] Running [Unit] tests for EnrollmentToken and QR renderer..."
cargo test -p umcf-enrollment --lib tests::unit_enrollment_token_generate_uri_roundtrip_ttl_and_qr_rendering -- --nocapture
echo "  -> [Unit] PASS"

echo "[2/5] Running [Integration] tests for Master-Candidate enrollment handshake..."
cargo test -p umcf-enrollment --lib tests::integration_full_master_qr_candidate_scan_and_cert_issuance_handshake -- --nocapture
if [ ! -f "$REPO_ROOT/artifacts/evidence/stage-02/sample_enrollment_qr.svg" ]; then
  echo "ERROR: sample_enrollment_qr.svg not found!"
  exit 1
fi
echo "  -> [Integration] PASS"

echo "[3/5] Running [Simulation] concurrent token replay attack fault-injection..."
cargo test -p umcf-enrollment --lib tests::simulation_concurrent_token_replay_rejected_by_cache -- --nocapture
echo "  -> [Simulation] PASS"

echo "[4/5] Running [Hardware-Required] hardware probe and quota check..."
bash "$SCRIPT_DIR/podman-quota-check.sh"
echo "  -> [Hardware-Required] PASS"

echo "[5/5] Running [Manually-Verified] sample_enrollment_qr.svg review..."
if grep -q "<svg" "$REPO_ROOT/artifacts/evidence/stage-02/sample_enrollment_qr.svg" && \
   grep -q "</svg>" "$REPO_ROOT/artifacts/evidence/stage-02/sample_enrollment_qr.svg"; then
  echo "  -> [Manually-Verified] PASS"
else
  echo "ERROR: invalid SVG format in sample_enrollment_qr.svg"
  exit 1
fi

echo "=== Subphase S2.P2.SP1: ALL TEST TIERS PASSED ==="
