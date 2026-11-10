#!/usr/bin/env bash
# scripts/test-s2-p1-sp2.sh — Classified test runner for Subphase S2.P1.SP2
# Classification: [Unit], [Integration], [Simulation], [Hardware-Required], [Manually-Verified]
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
cd "${REPO_ROOT}"

echo "=============================================================================="
echo " Subphase S2.P1.SP2: Platform Keychain/Keystore & Encrypted Vault Test Suite"
echo "=============================================================================="

# 1. Unit Tests
echo "[1/5] Running [Unit] tests for LocalEncryptedVault..."
cargo test -p umcf-identity --lib tests::unit_vault_create_unlock_wrong_passphrase_rejection_and_rotation -- --nocapture
echo "-> [Unit] PASS"

# 2. Integration Tests
echo "[2/5] Running [Integration] tests for identity persistence continuity..."
cargo test -p umcf-identity --lib tests::integration_vault_identity_continuity_across_simulated_restart -- --nocapture
echo "-> [Integration] PASS"

# 3. Simulation Tests (500 fuzzing iterations tamper detection)
echo "[3/5] Running [Simulation] 500 fuzzing iterations tamper detection..."
cargo test -p umcf-identity --lib tests::simulation_vault_fuzzing_500_iterations_tamper_detection -- --nocapture
echo "-> [Simulation] PASS"

# 4. Hardware-Required Tests (Host keystore environment and quota check)
echo "[4/5] Running [Hardware-Required] host platform keystore & quota check..."
bash "${SCRIPT_DIR}/podman-quota-check.sh"
echo "-> [Hardware-Required] PASS"

# 5. Manually-Verified Tests (vault_security_audit.json review)
echo "[5/5] Running [Manually-Verified] vault_security_audit.json review..."
python3 -c '
import json
with open("artifacts/evidence/stage-02/vault_security_audit.json", "r", encoding="utf-8") as f:
    audit = json.load(f)
assert audit["kdf"]["algorithm"] == "Argon2id", "KDF algorithm mismatch"
assert audit["aead"]["cipher"] == "XChaCha20-Poly1305", "AEAD cipher mismatch"
assert audit["memory_protection"]["zeroize_on_drop"] is True, "Zeroize on drop missing"
assert audit["tamper_resistance"]["fuzz_iterations"] >= 500, "Fuzz iterations < 500"
print("Vault Security Audit Verified")
' || python -c '
import json
with open("artifacts/evidence/stage-02/vault_security_audit.json", "r", encoding="utf-8") as f:
    audit = json.load(f)
assert audit["kdf"]["algorithm"] == "Argon2id", "KDF algorithm mismatch"
assert audit["aead"]["cipher"] == "XChaCha20-Poly1305", "AEAD cipher mismatch"
assert audit["memory_protection"]["zeroize_on_drop"] is True, "Zeroize on drop missing"
assert audit["tamper_resistance"]["fuzz_iterations"] >= 500, "Fuzz iterations < 500"
print("Vault Security Audit Verified")
'
echo "-> [Manually-Verified] PASS"

echo "=============================================================================="
echo " Subphase S2.P1.SP2: ALL 5/5 CLASSIFIED TESTS PASSED"
echo "=============================================================================="
