#!/usr/bin/env bash
# scripts/test-s2-p1-sp3.sh — Classified test runner for Subphase S2.P1.SP3
# Classification: [Unit], [Integration], [Simulation], [Hardware-Required], [Manually-Verified]
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
cd "${REPO_ROOT}"

echo "=============================================================================="
echo " Subphase S2.P1.SP3: Mutual Attestation & Signed Certificates Test Suite"
echo "=============================================================================="

# 1. Unit Tests
echo "[1/5] Running [Unit] tests for Cluster CA and CRL microbenchmark..."
cargo test -p umcf-identity --lib tests::unit_cluster_ca_generation_and_leaf_issuance -- --nocapture
cargo test -p umcf-identity --release --lib tests::unit_crl_lookup_latency_microbenchmark -- --nocapture
echo "-> [Unit] PASS"

# 2. Integration Tests
echo "[2/5] Running [Integration] tests for Mutual Attestation Protocol..."
cargo test -p umcf-identity --lib tests::integration_mutual_attestation_protocol_with_tls_exporter -- --nocapture
if [ ! -f "artifacts/evidence/stage-02/mtls_attestation_transcript.json" ]; then
    echo "ERROR: mtls_attestation_transcript.json missing"
    exit 1
fi
echo "-> [Integration] PASS"

# 3. Simulation Tests (Fault injection / MITM relay / replay)
echo "[3/5] Running [Simulation] MITM relay, replay, and revocation fault-injection..."
cargo test -p umcf-identity --lib tests::simulation_attestation_fault_injection_mitm_and_replay_rejection -- --nocapture
echo "-> [Simulation] PASS"

# 4. Hardware-Required Tests (Host quota check)
echo "[4/5] Running [Hardware-Required] hardware probe and quota check..."
bash "${SCRIPT_DIR}/podman-quota-check.sh"
echo "-> [Hardware-Required] PASS"

# 5. Manually-Verified Tests (mtls_attestation_transcript.json review)
echo "[5/5] Running [Manually-Verified] mtls_attestation_transcript.json review..."
python3 -c '
import json
with open("artifacts/evidence/stage-02/mtls_attestation_transcript.json", "r", encoding="utf-8") as f:
    t = json.load(f)
assert t["tls_exporter_label"] == "EXPORTER-UMCF-ATTEST-v1", "TLS exporter label mismatch"
assert t["challenger_did"].startswith("did:key:z6Mk"), "Challenger DID mismatch"
assert t["prover_did"].startswith("did:key:z6Mk"), "Prover DID mismatch"
assert t["verification_status"] == "SUCCESS", "Verification status not SUCCESS"
print("Mutual Attestation Transcript Verified")
' || python -c '
import json
with open("artifacts/evidence/stage-02/mtls_attestation_transcript.json", "r", encoding="utf-8") as f:
    t = json.load(f)
assert t["tls_exporter_label"] == "EXPORTER-UMCF-ATTEST-v1", "TLS exporter label mismatch"
assert t["challenger_did"].startswith("did:key:z6Mk"), "Challenger DID mismatch"
assert t["prover_did"].startswith("did:key:z6Mk"), "Prover DID mismatch"
assert t["verification_status"] == "SUCCESS", "Verification status not SUCCESS"
print("Mutual Attestation Transcript Verified")
'
echo "-> [Manually-Verified] PASS"

echo "=============================================================================="
echo " Subphase S2.P1.SP3: ALL 5/5 CLASSIFIED TESTS PASSED"
echo "=============================================================================="
