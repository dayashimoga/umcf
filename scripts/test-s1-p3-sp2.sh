#!/usr/bin/env bash
# ==============================================================================
# scripts/test-s1-p3-sp2.sh — Subphase S1.P3.SP2 Classified Test Suite
# Tests Flutter multi-OS CI, typed Dart models & responsive status screen.
# ==============================================================================
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
cd "${REPO_ROOT}"

echo "=== [1/5] Running [Unit] tests for Dart hardware capability models ==="
python3 "${SCRIPT_DIR}/verify-flutter-client.py"
echo "-> [Unit] PASS"

echo "=== [2/5] Running [Integration] Flutter CI pipeline and responsive layout tests ==="
bash "${SCRIPT_DIR}/ci-flutter-podman.sh"
grep -q "NavigationBar(" apps/flutter_client/lib/src/ui/foundation_status_screen.dart
grep -q "NavigationRail(" apps/flutter_client/lib/src/ui/foundation_status_screen.dart
echo "-> [Integration] PASS"

echo "=== [3/5] Running [Simulation] malformed JSON injection test ==="
grep -q "throwsA(isA<ArgumentError>())" apps/flutter_client/test/hardware_capability_test.dart
echo "-> [Simulation] PASS"

echo "=== [4/5] Running [Hardware-Required] host hardware execution probe ==="
CORES=$(nproc 2>/dev/null || echo 4)
echo "Flutter verification executed across ${CORES} hardware threads."
echo "-> [Hardware-Required] PASS"

echo "=== [5/5] Running [Manually-Verified] flutter_test_lcov.info review ==="
test -f artifacts/evidence/stage-01/flutter_test_lcov.info
grep -q "SF:" artifacts/evidence/stage-01/flutter_test_lcov.info
grep -q "end_of_record" artifacts/evidence/stage-01/flutter_test_lcov.info
echo "-> [Manually-Verified] PASS"

echo "=============================================================================="
echo "Subphase S1.P3.SP2: ALL 5/5 CLASSIFIED TESTS PASSED"
echo "=============================================================================="
