#!/usr/bin/env bash
# Enforces the mandatory <= 30 GB Podman storage allocation for UMCF on Linux/macOS.
set -euo pipefail

MAX_GIB=30
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
STORAGE_CONF="${SCRIPT_DIR}/../infra/podman/storage.conf"

if [[ ! -f "${STORAGE_CONF}" ]]; then
  echo "ERROR: Missing ${STORAGE_CONF}" >&2
  exit 1
fi

CONFIGURED_SIZE=$(grep -E '^size\s*=\s*"[0-9]+G"' "${STORAGE_CONF}" | sed -E 's/.*"([0-9]+)G".*/\1/' || true)
if [[ -z "${CONFIGURED_SIZE}" ]]; then
  echo "ERROR: storage.conf does not declare an explicit size = \"30G\" cap" >&2
  exit 1
fi

if (( CONFIGURED_SIZE > MAX_GIB )); then
  echo "ERROR: Configured Podman storage (${CONFIGURED_SIZE} GiB) exceeds ${MAX_GIB} GiB limit" >&2
  exit 1
fi

if command -v podman >/dev/null 2>&1; then
  echo "Podman detected: $(podman --version)"
  if podman machine list --format "{{.Name}} {{.DiskSize}}" >/dev/null 2>&1; then
    while read -r name disk; do
      if [[ "${disk}" =~ ^([0-9]+)GiB$ ]]; then
        gib="${BASH_REMATCH[1]}"
        if (( gib > MAX_GIB )); then
          echo "ERROR: Podman machine ${name} allocation (${gib} GiB) exceeds ${MAX_GIB} GiB limit" >&2
          exit 1
        fi
      fi
    done < <(podman machine list --noheading --format "{{.Name}} {{.DiskSize}}" 2>/dev/null || true)
  fi
fi

echo "PASS: Podman storage quota verified (<= ${MAX_GIB} GiB)."

