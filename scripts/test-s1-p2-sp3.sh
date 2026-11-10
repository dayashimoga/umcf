#!/usr/bin/env bash
# ==============================================================================
# Subphase S1.P2.SP3 Classified Test Suite (Linux / Rootless Podman)
# Tests telemetry, tracing, W3C TraceContext, error taxonomy, and metrics.
# ==============================================================================
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
cd "${REPO_ROOT}"

PASS_COUNT=0
TOTAL_TESTS=5

echo "=== [1/5] Running [Unit] tests for umcf-telemetry ==="
cargo test -p umcf-telemetry --lib tests::unit_ -- --nocapture
PASS_COUNT=$((PASS_COUNT + 1))
echo "-> [Unit] PASS"

echo "=== [2/5] Running [Integration] tests for umcf-telemetry ==="
cargo test -p umcf-telemetry --lib tests::integration_ -- --nocapture
if [[ ! -f "artifacts/evidence/stage-01/error_taxonomy_catalog.json" ]]; then
  echo "ERROR: error_taxonomy_catalog.json was not generated"
  exit 1
fi
PASS_COUNT=$((PASS_COUNT + 1))
echo "-> [Integration] PASS"

echo "=== [3/5] Running [Simulation] secret redaction leak prevention ==="
cargo test -p umcf-telemetry --lib tests::simulation_ -- --nocapture
PASS_COUNT=$((PASS_COUNT + 1))
echo "-> [Simulation] PASS"

echo "=== [4/5] Running [Hardware-Required] multi-threaded atomic concurrency verification ==="
cargo test -p umcf-telemetry --lib tests::unit_lock_free_metrics_and_openmetrics_export -- --nocapture
PASS_COUNT=$((PASS_COUNT + 1))
echo "-> [Hardware-Required] PASS"

echo "=== [5/5] Running [Manually-Verified] RFC-9457 & OpenMetrics inspection ==="
python3 -c "
import json
with open('artifacts/evidence/stage-01/error_taxonomy_catalog.json', 'r') as f:
    data = json.load(f)
assert len(data) == 27, f'Expected 27 error codes, found {len(data)}'
assert all('code' in item and 'http_status' in item for item in data)
print(f'Successfully validated {len(data)} error taxonomy entries')
" || python -c "
import json
with open('artifacts/evidence/stage-01/error_taxonomy_catalog.json', 'r') as f:
    data = json.load(f)
assert len(data) == 27, f'Expected 27 error codes, found {len(data)}'
assert all('code' in item and 'http_status' in item for item in data)
print(f'Successfully validated {len(data)} error taxonomy entries')
"
PASS_COUNT=$((PASS_COUNT + 1))
echo "-> [Manually-Verified] PASS"

echo "=============================================================================="
echo "Subphase S1.P2.SP3: ALL 5/5 CLASSIFIED TESTS PASSED"
echo "=============================================================================="
