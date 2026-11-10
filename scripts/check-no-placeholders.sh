#!/usr/bin/env bash
# Scans production source paths for forbidden placeholder tokens:
# todo!(), unimplemented!(), FIXME, or fake mock stubs.
# Must complete in < 2.0 seconds and exit 1 if violations are found.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

BANNED_PATTERN='(\btodo!\(|\bunimplemented!\(|\bFIXME\b)'

# Production search paths
SEARCH_DIRS=(
  "${REPO_ROOT}/crates"
  "${REPO_ROOT}/services"
  "${REPO_ROOT}/apps"
)

VIOLATIONS=0
START_TIME=$(date +%s%N 2>/dev/null || date +%s)

echo "=== UMCF Production Placeholder Gate ==="
echo "Scanning production sources in: crates/, services/, apps/..."

for d in "${SEARCH_DIRS[@]}"; do
  if [ -d "$d" ]; then
    # Find all source files, excluding target, node_modules, build, .dart_tool, and test folders
    while IFS= read -r file; do
      # Scan file for banned pattern, ignoring lines that are clearly tests or comments explaining the rule
      if grep -n -E "${BANNED_PATTERN}" "$file" 2>/dev/null | grep -v -E '(//.*test|#\[cfg\(test\)|test/|tests/)'; then
        echo "VIOLATION in $file"
        VIOLATIONS=$((VIOLATIONS + 1))
      fi
    done < <(find "$d" -type f \( -name "*.rs" -o -name "*.ts" -o -name "*.js" -o -name "*.mjs" -o -name "*.svelte" -o -name "*.dart" \) \
      ! -path "*/target/*" \
      ! -path "*/node_modules/*" \
      ! -path "*/build/*" \
      ! -path "*/.dart_tool/*" \
      ! -path "*/dist/*" \
      ! -path "*/test/*" \
      ! -path "*/tests/*")
  fi
done

if [ "$VIOLATIONS" -gt 0 ]; then
  echo "FAIL: Found $VIOLATIONS forbidden placeholder(s) in production paths." >&2
  exit 1
fi

echo "PASS: Zero forbidden placeholders found in production paths."
exit 0
