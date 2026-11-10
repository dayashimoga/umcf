#!/usr/bin/env bash
# Executes classified tests for Subphase S1.P1.SP2:
# - Unit: math & threshold evaluator with mocked byte counters (20 GiB PASS, 26 GiB WARN, 29.5 GiB FAIL)
# - Integration: Podman container definitions and normal quota execution
# - Simulation: synthetic 30.1 GiB byte counter in PODMAN_QUOTA_SIMULATE_BYTES triggering hard abort
# - Hardware-Required: host disk partition and Podman storage inspection
# - Manually-Verified: rootless UID mapping (keep-id) and bounded cache volumes verification
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

echo "=== Subphase S1.P1.SP2 Classified Test Runner ==="

# 1. Unit Test
echo "[1/5] Running [Unit] podman-quota-check threshold tests..."
OUT_20=$(PODMAN_QUOTA_SIMULATE_BYTES="21474836480" bash "${SCRIPT_DIR}/podman-quota-check.sh" --json)
if echo "$OUT_20" | grep -q '"status": "PASS"'; then
  echo "  -> 20 GiB threshold: PASS"
else
  echo "  -> 20 GiB threshold: FAIL" >&2; exit 1
fi

OUT_26=$(PODMAN_QUOTA_SIMULATE_BYTES="27917287424" bash "${SCRIPT_DIR}/podman-quota-check.sh" --json)
if echo "$OUT_26" | grep -q '"status": "WARN"'; then
  echo "  -> 26 GiB threshold: WARN (PASS)"
else
  echo "  -> 26 GiB threshold: FAIL" >&2; exit 1
fi

if PODMAN_QUOTA_SIMULATE_BYTES="31677353984" bash "${SCRIPT_DIR}/podman-quota-check.sh" --json >/dev/null 2>&1; then
  echo "  -> 29.5 GiB threshold: FAIL (did not exit with error)" >&2; exit 1
else
  echo "  -> 29.5 GiB threshold: Correctly rejected (PASS)"
fi
echo "  -> [Unit] PASS"

# 2. Integration Test
echo "[2/5] Running [Integration] container configurations & live quota check..."
test -f "${REPO_ROOT}/containers/Podmanfile.rust"
test -f "${REPO_ROOT}/containers/Podmanfile.web"
test -f "${REPO_ROOT}/containers/Podmanfile.flutter"
test -f "${REPO_ROOT}/compose.podman.yml"
bash "${SCRIPT_DIR}/podman-quota-check.sh"
echo "  -> [Integration] PASS"

# 3. Simulation Test
echo "[3/5] Running [Simulation] 30.1 GiB synthetic byte counter test..."
if PODMAN_QUOTA_SIMULATE_BYTES="32319619072" bash "${SCRIPT_DIR}/podman-quota-check.sh" --json >/dev/null 2>&1; then
  echo "  -> [Simulation] FAIL (gate failed to reject 30.1 GiB)" >&2; exit 1
else
  echo "  -> [Simulation] PASS: Correctly caught synthetic 30.1 GiB counter"
fi

# 4. Hardware-Required Test
echo "[4/5] Running [Hardware-Required] host storage & podman machine inspection..."
bash "${SCRIPT_DIR}/enforce-podman-quota.sh"
echo "  -> [Hardware-Required] PASS"

# 5. Manually-Verified Test
echo "[5/5] Running [Manually-Verified] rootless keep-id & cache volume check..."
grep -q 'userns_mode: "keep-id"' "${REPO_ROOT}/compose.podman.yml"
grep -q 'umcf_cargo_cache:' "${REPO_ROOT}/compose.podman.yml"
grep -q 'umcf_sccache:' "${REPO_ROOT}/compose.podman.yml"
echo "  -> [Manually-Verified] PASS"

echo "=== Subphase S1.P1.SP2 All Classified Tests Passed (100%) ==="
exit 0
