#!/usr/bin/env bash
# scripts/test-s2-p1-sp1.sh — Classified test runner for Subphase S2.P1.SP1
# Classification: [Unit], [Integration], [Simulation], [Hardware-Required], [Manually-Verified]
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
cd "${REPO_ROOT}"

echo "=============================================================================="
echo " Subphase S2.P1.SP1: Node Identity & DID Generation Classified Test Suite"
echo "=============================================================================="

# 1. Unit Tests
echo "[1/5] Running [Unit] tests for umcf-identity..."
cargo test -p umcf-identity --lib tests::unit_ -- --nocapture
echo "-> [Unit] PASS"

# 2. Integration Tests
echo "[2/5] Running [Integration] tests for WireEnvelope auth..."
cargo test -p umcf-identity --lib tests::integration_ -- --nocapture
if [ ! -f "artifacts/evidence/stage-02/did_document_sample.json" ]; then
    echo "ERROR: did_document_sample.json missing"
    exit 1
fi
echo "-> [Integration] PASS"

# 3. Simulation Tests (Fault injection / tampering)
echo "[3/5] Running [Simulation] tampering and impersonation rejection..."
cargo test -p umcf-identity --lib tests::simulation_ -- --nocapture
echo "-> [Simulation] PASS"

# 4. Hardware-Required Tests (Release microbenchmark)
echo "[4/5] Running [Hardware-Required] release sign/verify microbenchmark (< 80 µs)..."
cargo test -p umcf-identity --release --lib tests::unit_envelope_signing_and_verification_latency_microbenchmark -- --nocapture
echo "-> [Hardware-Required] PASS"

# 5. Manually-Verified Tests (W3C DID Document Core 1.0 inspection)
echo "[5/5] Running [Manually-Verified] did_document_sample.json review..."
python3 -c '
import json, sys
with open("artifacts/evidence/stage-02/did_document_sample.json", "r", encoding="utf-8") as f:
    doc = json.load(f)
assert doc["id"].startswith("did:key:z6Mk"), "Invalid DID format"
assert len(doc["@context"]) >= 3, "Missing @context"
assert doc["verificationMethod"][0]["type"] == "Ed25519VerificationKey2020", "Missing Ed25519 verificationMethod"
assert doc["keyAgreement"][0]["type"] == "X25519KeyAgreementKey2020", "Missing X25519 keyAgreement"
print("DID Document Schema Verified")
' || python -c '
import json, sys
with open("artifacts/evidence/stage-02/did_document_sample.json", "r", encoding="utf-8") as f:
    doc = json.load(f)
assert doc["id"].startswith("did:key:z6Mk"), "Invalid DID format"
assert len(doc["@context"]) >= 3, "Missing @context"
assert doc["verificationMethod"][0]["type"] == "Ed25519VerificationKey2020", "Missing Ed25519 verificationMethod"
assert doc["keyAgreement"][0]["type"] == "X25519KeyAgreementKey2020", "Missing X25519 keyAgreement"
print("DID Document Schema Verified")
'
echo "-> [Manually-Verified] PASS"

echo "=============================================================================="
echo " Subphase S2.P1.SP1: ALL 5/5 CLASSIFIED TESTS PASSED"
echo "=============================================================================="
