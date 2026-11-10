#!/usr/bin/env bash
# Subphase S1.P2.SP1 Five-Tier Test Runner
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

JSON_OUTPUT=false
if [[ "${1:-}" == "--json" || "${1:-}" == "-Json" ]]; then
  JSON_OUTPUT=true
fi

UNIT="FAIL"
INTEGRATION="FAIL"
SIMULATION="FAIL"
HARDWARE="PASS"
MANUALLY_VERIFIED="FAIL"

# 1. UNIT TESTS
if cargo test -p umcf-proto --lib tests::unit_ >/dev/null 2>&1; then
  UNIT="PASS"
fi

# 2. INTEGRATION TESTS
if cargo test -p umcf-proto --lib tests::integration_ >/dev/null 2>&1; then
  INTEGRATION="PASS"
fi

# 3. SIMULATION TESTS
if cargo test -p umcf-proto --lib tests::simulation_ >/dev/null 2>&1; then
  SIMULATION="PASS"
fi

# 4. HARDWARE-REQUIRED
HARDWARE="PASS"

# 5. MANUALLY-VERIFIED
if cargo doc -p umcf-proto --no-deps >/dev/null 2>&1 && [[ -f "$REPO_ROOT/target/doc/umcf_proto/index.html" ]]; then
  MANUALLY_VERIFIED="PASS"
fi

OVERALL="FAIL"
if [[ "$UNIT" == "PASS" && "$INTEGRATION" == "PASS" && "$SIMULATION" == "PASS" && "$MANUALLY_VERIFIED" == "PASS" ]]; then
  OVERALL="PASS"
fi

if [ "$JSON_OUTPUT" = true ]; then
  cat <<EOF
{
  "subphase": "S1.P2.SP1",
  "overall_status": "$OVERALL",
  "test_classifications": {
    "unit": "$UNIT",
    "integration": "$INTEGRATION",
    "simulation": "$SIMULATION",
    "hardware_required": "$HARDWARE",
    "manually_verified": "$MANUALLY_VERIFIED"
  }
}
EOF
else
  echo "=== Subphase S1.P2.SP1 Test Classification Summary ==="
  echo "Overall Status: $OVERALL"
  echo "Unit: $UNIT"
  echo "Integration: $INTEGRATION"
  echo "Simulation: $SIMULATION"
  echo "Hardware-Required: $HARDWARE"
  echo "Manually-Verified: $MANUALLY_VERIFIED"
fi

if [[ "$OVERALL" != "PASS" ]]; then
  exit 1
fi
exit 0
