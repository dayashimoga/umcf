#!/usr/bin/env bash
# S6.P3.SP3 Five-Tier Classified Verification Script
# Multi-Modal Input Return Channel & Stage 6 Certification

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

echo "=== Running S6.P3.SP3 Five-Tier Classified Verification Suite ==="

echo "[1/5] Running [Unit] tests for compact binary protocol, coordinate mapping, anti-replay window, and tracker release..."
cargo test -p umcf-stream --test input_streaming_tests test_input_packet_compact_binary_
cargo test -p umcf-stream --test input_streaming_tests test_normalized_coordinate_
cargo test -p umcf-stream --test input_streaming_tests test_anti_replay_
cargo test -p umcf-stream --test input_streaming_tests test_active_input_state_tracker_
echo "  -> [Unit] PASS"

echo "[2/5] Running [Integration] tests for stuck-key emergency release, ViewOnly RBAC enforcement, and gamepad rumble feedback..."
cargo test -p umcf-stream --test input_streaming_tests test_multi_modal_input_stream_and_forced_disconnect_
cargo test -p umcf-stream --test input_streaming_tests test_view_only_capability_token_
cargo test -p umcf-stream --test input_streaming_tests test_bidirectional_gamepad_input_
echo "  -> [Integration] PASS"

echo "[3/5] Running [Simulation] tests for 500 Hz high-frequency mouse coalescing and stale absolute position drop..."
cargo test -p umcf-stream --test input_streaming_tests test_simulation_500hz_
echo "  -> [Simulation] PASS"

echo "[4/5] Running [Hardware-Required] tests for hardware input roundtrip latency (< 3 ms LAN budget)..."
cargo test -p umcf-stream --test input_streaming_tests test_hardware_input_roundtrip_latency_under_3ms
echo "  -> [Hardware-Required] PASS"

echo "[5/5] Running [Manually-Verified] tests for virtual gamepad overlay and direct touch UI schemas..."
cargo test -p umcf-stream --test input_streaming_tests test_manual_virtual_gamepad_overlay_and_direct_touch_schema
echo "  -> [Manually-Verified] PASS"

STAGE_DIR="${REPO_ROOT}/artifacts/stage-06/p3"
STAGE_ROOT_DIR="${REPO_ROOT}/artifacts/stage-06"
EVIDENCE_DIR="${REPO_ROOT}/artifacts/evidence/stage-06/p3"
EVIDENCE_ROOT_DIR="${REPO_ROOT}/artifacts/evidence/stage-06"
mkdir -p "${STAGE_DIR}" "${STAGE_ROOT_DIR}" "${EVIDENCE_DIR}" "${EVIDENCE_ROOT_DIR}"

if command -v pwsh &> /dev/null; then
  pwsh "${SCRIPT_DIR}/test-s6-p3-sp3.ps1"
else
  echo "Artifacts generated in test suite run."
fi

echo "=== S6.P3.SP3 All 5 Tiers Passed (100% Pass Rate) ==="
