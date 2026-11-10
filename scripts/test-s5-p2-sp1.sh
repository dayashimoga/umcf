#!/usr/bin/env bash
# Test script for Subphase S5.P2.SP1: Safe Rust FFI Bindings to llama.cpp & GGML Backend Loader
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

echo "=== Running S5.P2.SP1 Test Harness ==="
cargo test -p umcf-llama-sys
cargo test -p umcf-ai --test ffi_and_loader_tests

mkdir -p "${REPO_ROOT}/artifacts/stage-05/p2"
mkdir -p "${REPO_ROOT}/artifacts/evidence/stage-05/p2"

echo "S5.P2.SP1 tests passed successfully."
