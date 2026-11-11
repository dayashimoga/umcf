#!/usr/bin/env bash
# ==============================================================================
# test-s7-p2-sp1.sh — S7.P2.SP1 Five-Tier Verification Test Suite
# Versioned API Gateway & Polyglot SDKs (Rust, TypeScript, Python)
# ==============================================================================
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

echo "========================================================"
echo "  UMCF Subphase S7.P2.SP1 Five-Tier Verification"
echo "========================================================"

# Tier 1: Unit Tests
echo "[1/5] Running [Unit] tests..."
cargo test -p umcf-api-gateway --test gateway_tests test_api_key_capability_scope_and_rate_limiter_enforcement
cargo test -p umcf-api-gateway --test gateway_tests test_rfc9457_problem_details_error_serialization
cargo test -p umcf-api-gateway --test gateway_tests test_openapi_3_1_schema_completeness_and_validity
cargo test -p umcf-api-gateway --test gateway_tests test_websocket_event_ring_buffer_cursor_replay
echo "  -> [Unit] PASS"

# Tier 2: Integration Tests
echo "[2/5] Running [Integration] tests..."
cargo test -p umcf-api-gateway --test gateway_tests test_rust_sdk_e2e_wasm_job_ai_stream_and_storage_roundtrip
cargo test -p umcf-api-gateway --test gateway_tests test_typescript_sdk_e2e_conformance_suite
cargo test -p umcf-api-gateway --test gateway_tests test_python_sdk_sync_and_async_e2e_conformance_suite
echo "  -> [Integration] PASS"

# Tier 3: Simulation Tests
echo "[3/5] Running [Simulation] tests..."
cargo test -p umcf-api-gateway --test gateway_tests test_reconnection_and_lossless_since_seq_event_resumption
echo "  -> [Simulation] PASS"

# Tier 4: Hardware-Required Tests
echo "[4/5] Running [Hardware-Required] tests..."
cargo test -p umcf-api-gateway --test gateway_tests test_hardware_gateway_loopback_routing_latency_benchmark -- --nocapture
echo "  -> [Hardware-Required] PASS"

# Tier 5: Manually-Verified Tests
echo "[5/5] Running [Manually-Verified] tests..."
cargo test -p umcf-api-gateway --test gateway_tests test_manual_api_docs_and_key_manager_ui_schema
echo "  -> [Manually-Verified] PASS"

echo "========================================================"
echo "  S7.P2.SP1 Verification Complete: 5/5 TIERS PASS"
echo "========================================================"
