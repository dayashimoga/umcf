<#
.SYNOPSIS
  Verifies Podman storage quota is below 29.0 GiB (31,138,512,896 bytes) for pre-build safety,
  and enforces maximum 30 GiB ceiling.
#>
param(
  [switch]$Json
)

$ErrorActionPreference = "Stop"
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path

# Run enforce-podman-quota.ps1
$enforceScript = Join-Path $ScriptDir "enforce-podman-quota.ps1"
if (Test-Path $enforceScript) {
  if ($Json) {
    & powershell -ExecutionPolicy Bypass -File $enforceScript -Json
  } else {
    & powershell -ExecutionPolicy Bypass -File $enforceScript
  }
} else {
  Write-Error "Missing enforce-podman-quota.ps1"
  exit 1
}
