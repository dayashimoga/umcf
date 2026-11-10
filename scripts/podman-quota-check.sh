#!/usr/bin/env bash
# Hard Disk Quota Enforcement Script for UMCF Podman Environment
# Enforces:
#   - Soft Warning at 25.0 GiB (26,843,545,600 bytes)
#   - Hard Abort & Auto-Prune at 29.0 GiB (31,138,512,896 bytes)
#   - Hard 30.0 GiB ceiling (32,212,254,720 bytes)
# Outputs valid JSON with schema:
#   { "images_bytes": ..., "volumes_bytes": ..., "target_bytes": ..., "total_bytes": ..., "limit_bytes": 31138512896, "status": "PASS" }
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

LIMIT_BYTES=31138512896    # 29.0 GiB
WARN_BYTES=26843545600     # 25.0 GiB

JSON_OUTPUT=false
for arg in "$@"; do
  if [ "$arg" == "--json" ]; then
    JSON_OUTPUT=true
  fi
done

IMAGES_BYTES=0
VOLUMES_BYTES=0
TARGET_BYTES=0

# Calculate target directory size if present
if [ -d "${REPO_ROOT}/target" ]; then
  TARGET_BYTES=$(du -sb "${REPO_ROOT}/target" 2>/dev/null | awk '{print $1}' || du -sk "${REPO_ROOT}/target" 2>/dev/null | awk '{print $1 * 1024}' || echo 0)
fi

# Query Podman storage via system df if podman is installed
if command -v podman >/dev/null 2>&1; then
  DF_JSON=$(podman system df --format json 2>/dev/null || echo "{}")
  if [ -n "$DF_JSON" ] && [ "$DF_JSON" != "{}" ]; then
    # Parse images and volumes size if jq/python is available
    if command -v python3 >/dev/null 2>&1; then
      SIZES=$(python3 -c "
import json, sys
try:
    data = json.loads('''$DF_JSON''')
    imgs = sum(item.get('Size', 0) if isinstance(item.get('Size'), int) else 0 for item in data.get('Images', []))
    vols = sum(item.get('Size', 0) if isinstance(item.get('Size'), int) else 0 for item in data.get('Volumes', []))
    print(f'{imgs} {vols}')
except Exception:
    print('0 0')
" 2>/dev/null || echo "0 0")
      IMAGES_BYTES=$(echo "$SIZES" | awk '{print $1}')
      VOLUMES_BYTES=$(echo "$SIZES" | awk '{print $2}')
    fi
  fi
fi

# Check for synthetic fault injection simulation
if [ -n "${PODMAN_QUOTA_SIMULATE_BYTES:-}" ]; then
  TOTAL_BYTES="$PODMAN_QUOTA_SIMULATE_BYTES"
else
  TOTAL_BYTES=$((IMAGES_BYTES + VOLUMES_BYTES + TARGET_BYTES))
fi

STATUS="PASS"
PRUNED=false

if [ "$TOTAL_BYTES" -ge "$LIMIT_BYTES" ]; then
  STATUS="FAIL"
  # Hard abort threshold reached: trigger automated cache prune attempt
  if command -v podman >/dev/null 2>&1; then
    podman system prune -f >/dev/null 2>&1 || true
    PRUNED=true
  fi
elif [ "$TOTAL_BYTES" -ge "$WARN_BYTES" ]; then
  STATUS="WARN"
fi

if [ "$JSON_OUTPUT" = true ]; then
  cat <<EOF
{
  "images_bytes": ${IMAGES_BYTES},
  "volumes_bytes": ${VOLUMES_BYTES},
  "target_bytes": ${TARGET_BYTES},
  "total_bytes": ${TOTAL_BYTES},
  "limit_bytes": ${LIMIT_BYTES},
  "status": "${STATUS}"
}
EOF
else
  echo "=== UMCF Podman Storage Quota Enforcer ==="
  echo "Images Bytes   : ${IMAGES_BYTES}"
  echo "Volumes Bytes  : ${VOLUMES_BYTES}"
  echo "Target Bytes   : ${TARGET_BYTES}"
  echo "Total Bytes    : ${TOTAL_BYTES} / ${LIMIT_BYTES} (limit: 29.0 GiB)"
  echo "Status         : ${STATUS}"
  if [ "$PRUNED" = true ]; then
    echo "Notice         : Automated cache prune routine triggered."
  fi
fi

if [ "$STATUS" = "FAIL" ]; then
  exit 1
fi
exit 0
