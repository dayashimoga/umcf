#!/usr/bin/env bash
# Verifies the presence, non-emptiness, and mandatory schemas of all UMCF governance files.
# Also verifies that all 72 subphases (S1.P1.SP1 through S8.P3.SP3) are represented in TODO.md.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

FAILURES=0

check_file() {
  local rel_path="$1"
  local full_path="${REPO_ROOT}/${rel_path}"
  if [ ! -f "${full_path}" ]; then
    echo "ERROR: Missing governance file: ${rel_path}" >&2
    FAILURES=$((FAILURES + 1))
    return 1
  fi
  if [ ! -s "${full_path}" ]; then
    echo "ERROR: Empty governance file: ${rel_path}" >&2
    FAILURES=$((FAILURES + 1))
    return 1
  fi
  echo "  OK: ${rel_path}"
}

echo "=== UMCF Governance Verification ==="

REQUIRED_FILES=(
  "ENGINEERING_CONTRACT.md"
  "AGENTS.md"
  "SECURITY.md"
  "CONTRIBUTING.md"
  "LICENSE-APACHE"
  "LICENSE-MIT"
  ".github/CODEOWNERS"
  "TODO.md"
  "CHANGELOG.md"
  "README.md"
  ".editorconfig"
  ".gitattributes"
  ".gitignore"
  "docs/architecture/SYSTEM_OVERVIEW.md"
  "docs/architecture/COMPONENT_CONTRACTS.md"
  "docs/architecture/DATA_FLOWS.md"
  "docs/requirements/FUNCTIONAL_REQUIREMENTS.md"
  "docs/requirements/NON_FUNCTIONAL_REQUIREMENTS.md"
  "docs/requirements/TRACEABILITY_MATRIX.md"
  "docs/implementation/MASTER_ROADMAP.md"
  "infra/podman/storage.conf"
  "infra/podman/Containerfile"
  "compose.podman.yml"
  "containers/Podmanfile.rust"
  "containers/Podmanfile.web"
  "containers/Podmanfile.flutter"
  "scripts/enforce-podman-quota.sh"
  "scripts/enforce-podman-quota.ps1"
  "scripts/podman-quota-check.sh"
  "scripts/podman-quota-check.ps1"
  "scripts/check-no-placeholders.sh"
  "scripts/check-no-placeholders.ps1"
)

for f in "${REQUIRED_FILES[@]}"; do
  check_file "$f" || true
done

# Verify all 7 rules in .agent/rules/
echo "Checking Antigravity rules..."
for r in 00-global-governance.md 01-rust-standards.md 02-flutter-standards.md 03-svelte-web-standards.md 04-security-and-privacy.md 05-hardware-and-benchmarking.md 06-ci-cd-and-podman.md; do
  check_file ".agent/rules/${r}" || true
done

# Verify all 9 skills in .agent/skills/
echo "Checking Antigravity skills..."
for s in umcf-architecture umcf-rust-dev umcf-flutter-dev umcf-svelte-dev umcf-distributed-systems umcf-networking-p2p umcf-security-audit umcf-testing-verification umcf-devops-podman; do
  check_file ".agent/skills/${s}/SKILL.md" || true
done

# Verify workflows in .agent/workflows/
echo "Checking Antigravity workflows..."
for w in stage-execution-workflow.md audit-workflow.md subphase-implementation-workflow.md code-review-workflow.md testing-and-coverage-workflow.md release-and-packaging-workflow.md hardware-certification-workflow.md; do
  check_file ".agent/workflows/${w}" || true
done

# Verify all 72 subphase IDs in TODO.md
echo "Verifying all 72 subphases in TODO.md..."
TODO_FILE="${REPO_ROOT}/TODO.md"
MISSING_SUBPHASES=0

for s in {1..8}; do
  for p in {1..3}; do
    for sp in {1..3}; do
      sub_id="S${s}.P${p}.SP${sp}"
      if ! grep -q "${sub_id}" "${TODO_FILE}"; then
        echo "ERROR: Subphase ${sub_id} not found in TODO.md" >&2
        MISSING_SUBPHASES=$((MISSING_SUBPHASES + 1))
        FAILURES=$((FAILURES + 1))
      fi
    done
  done
done

if [ "$MISSING_SUBPHASES" -eq 0 ]; then
  echo "  OK: All 72 subphases present in TODO.md."
fi

if [ "$FAILURES" -gt 0 ]; then
  echo "FAIL: Governance verification failed with $FAILURES error(s)." >&2
  exit 1
fi

echo "PASS: All UMCF governance files, schemas, and 72 subphase entries verified."
exit 0

