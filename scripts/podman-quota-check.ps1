<#
.SYNOPSIS
  Hard Disk Quota Enforcement Script for UMCF Podman Environment
  Enforces:
    - Soft Warning at 25.0 GiB (26,843,545,600 bytes)
    - Hard Abort & Auto-Prune at 29.0 GiB (31,138,512,896 bytes)
    - Hard 30.0 GiB ceiling (32,212,254,720 bytes)
  Outputs valid JSON with schema:
    { "images_bytes": ..., "volumes_bytes": ..., "target_bytes": ..., "total_bytes": ..., "limit_bytes": 31138512896, "status": "PASS" }
#>
param(
  [switch]$Json
)

$ErrorActionPreference = "Stop"
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$RepoRoot = (Resolve-Path (Join-Path $ScriptDir "..")).Path

$LimitBytes = 31138512896L # 29.0 GiB
$WarnBytes  = 26843545600L # 25.0 GiB

$imagesBytes  = 0L
$volumesBytes = 0L
$targetBytes  = 0L

# 1. Target directory bytes
$targetPath = Join-Path $RepoRoot "target"
if (Test-Path $targetPath) {
  try {
    $targetBytes = (Get-ChildItem -Path $targetPath -Recurse -File -Force -ErrorAction SilentlyContinue |
      Measure-Object -Property Length -Sum).Sum
    if ($null -eq $targetBytes) { $targetBytes = 0L }
  } catch {
    $targetBytes = 0L
  }
}

# 2. Query Podman system df if podman is installed
$podmanCmd = Get-Command podman -ErrorAction SilentlyContinue
if ($null -ne $podmanCmd) {
  try {
    $dfJsonRaw = (& podman system df --format json 2>$null) | Out-String
    if ($LASTEXITCODE -eq 0 -and -not [string]::IsNullOrWhiteSpace($dfJsonRaw)) {
      $dfObj = $dfJsonRaw | ConvertFrom-Json
      if ($null -ne $dfObj.Images) {
        foreach ($img in $dfObj.Images) {
          if ($img.Size -is [int64] -or $img.Size -is [double] -or $img.Size -is [int]) {
            $imagesBytes += [int64]$img.Size
          }
        }
      }
      if ($null -ne $dfObj.Volumes) {
        foreach ($vol in $dfObj.Volumes) {
          if ($vol.Size -is [int64] -or $vol.Size -is [double] -or $vol.Size -is [int]) {
            $volumesBytes += [int64]$vol.Size
          }
        }
      }
    }
  } catch {
    # Podman VM or daemon offline, keep defaults
  }
}

# 3. Check for synthetic fault injection simulation
$simulated = $env:PODMAN_QUOTA_SIMULATE_BYTES
if (-not [string]::IsNullOrWhiteSpace($simulated)) {
  $totalBytes = [int64]$simulated
} else {
  $totalBytes = $imagesBytes + $volumesBytes + $targetBytes
}

$status = "PASS"
$pruned = $false

if ($totalBytes -ge $LimitBytes) {
  $status = "FAIL"
  # Hard abort: trigger automated cache prune attempt
  if ($null -ne $podmanCmd) {
    try {
      & podman system prune -f 2>$null | Out-Null
      $pruned = $true
    } catch {}
  }
} elseif ($totalBytes -ge $WarnBytes) {
  $status = "WARN"
}

$output = [ordered]@{
  images_bytes  = $imagesBytes
  volumes_bytes = $volumesBytes
  target_bytes  = $targetBytes
  total_bytes   = $totalBytes
  limit_bytes   = $LimitBytes
  status        = $status
}

if ($Json) {
  $output | ConvertTo-Json -Depth 4
} else {
  Write-Host "=== UMCF Podman Storage Quota Enforcer ==="
  Write-Host "Images Bytes   : $($imagesBytes)"
  Write-Host "Volumes Bytes  : $($volumesBytes)"
  Write-Host "Target Bytes   : $($targetBytes)"
  Write-Host "Total Bytes    : $($totalBytes) / $($LimitBytes) (limit: 29.0 GiB)"
  Write-Host "Status         : $($status)"
  if ($pruned) {
    Write-Host "Notice         : Automated cache prune routine triggered." -ForegroundColor Yellow
  }
}

if ($status -eq "FAIL") {
  exit 1
}
exit 0
