#!/usr/bin/env bash
# ==============================================================================
# scripts/generate-sbom-and-manifest.sh — UMCF SBOM & Release Manifest Generator
# Subphase S1.P3.SP3
# ==============================================================================
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
cd "${REPO_ROOT}"

python3 "${SCRIPT_DIR}/generate_sbom_and_manifest.py" || python "${SCRIPT_DIR}/generate_sbom_and_manifest.py"
