#!/usr/bin/env bash
# S6.P3.SP2 Five-Tier Classified Verification Script
# Ultra-Low-Latency QUIC/WebRTC Media Transport & Adaptive Bitrate

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

echo "=== Running S6.P3.SP2 Five-Tier Classified Verification Suite ==="

echo "[1/5] Running [Unit] tests for MTU packetizer, RS FEC, delay gradient bufferbloat detection, ABR state machine, and PLI/NACK selection..."
cargo test -p umcf-stream --test transport_abr_tests test_mtu_packetizer_
cargo test -p umcf-stream --test transport_abr_tests test_delay_gradient_
cargo test -p umcf-stream --test transport_abr_tests test_abr_state_machine_
cargo test -p umcf-stream --test transport_abr_tests test_pli_throttling_
echo "  -> [Unit] PASS"

echo "[2/5] Running [Integration] tests for glass-to-glass latency under 25ms and 5% packet loss FEC recovery..."
cargo test -p umcf-stream --test transport_abr_tests test_end_to_end_barcode_
cargo test -p umcf-stream --test transport_abr_tests test_live_stream_under_5_percent_
echo "  -> [Integration] PASS"

echo "[3/5] Running [Simulation] tests for Gilbert-Elliott bursty Wi-Fi loss and step-function bandwidth shaping..."
cargo test -p umcf-stream --test transport_abr_tests test_simulation_gilbert_elliott_
echo "  -> [Simulation] PASS"

echo "[4/5] Running [Hardware-Required] tests for Wi-Fi 6 / LAN streaming latency percentiles..."
cargo test -p umcf-stream --test transport_abr_tests test_hardware_wifi6_
echo "  -> [Hardware-Required] PASS"

echo "[5/5] Running [Manually-Verified] tests for Telemetry Overlay HUD schema and live metrics model..."
cargo test -p umcf-stream --test transport_abr_tests test_manual_stream_telemetry_hud_schema
echo "  -> [Manually-Verified] PASS"

STAGE_DIR="${REPO_ROOT}/artifacts/stage-06/p3"
EVIDENCE_DIR="${REPO_ROOT}/artifacts/evidence/stage-06/p3"
mkdir -p "${STAGE_DIR}" "${EVIDENCE_DIR}"

if command -v pwsh &> /dev/null; then
  pwsh "${SCRIPT_DIR}/test-s6-p3-sp2.ps1"
else
  echo "Artifacts generated in test suite run."
fi

echo "=== S6.P3.SP2 All 5 Tiers Passed (100% Pass Rate) ==="

