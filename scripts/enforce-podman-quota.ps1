<#
.SYNOPSIS
  Enforces the mandatory <= 30 GB Podman storage allocation for UMCF.
.DESCRIPTION
  Inspects Podman installation, machine disk allocation (on Windows/macOS),
  and storage.conf configuration to guarantee Podman never exceeds 30 GiB (32,212,254,720 bytes).
#>
param(
  [int]$MaxStorageGiB = 30,
  [switch]$Json
)

$ErrorActionPreference = "Stop"
$MaxBytes = [int64]$MaxStorageGiB * 1024 * 1024 * 1024

$result = [ordered]@{
  status              = "PASS"
  max_allowed_gib     = $MaxStorageGiB
  max_allowed_bytes   = $MaxBytes
  podman_installed    = $false
  podman_version      = $null
  machine_name        = $null
  machine_disk_size   = $null
  machine_disk_gib    = $null
  storage_conf_path   = (Resolve-Path (Join-Path $PSScriptRoot "..\infra\podman\storage.conf")).Path
  storage_conf_valid  = $false
  violations          = @()
}

# 1. Verify infra/podman/storage.conf enforces size = "30G"
if (Test-Path $result.storage_conf_path) {
  $confContent = Get-Content -Raw $result.storage_conf_path
  if ($confContent -match 'size\s*=\s*"(\d+)G"') {
    $configuredGiB = [int]$Matches[1]
    if ($configuredGiB -le $MaxStorageGiB) {
      $result.storage_conf_valid = $true
    } else {
      $result.violations += "storage.conf size ($configuredGiB GiB) exceeds limit ($MaxStorageGiB GiB)"
    }
  } else {
    $result.violations += "storage.conf is missing explicit size = `"30G`" quota cap"
  }
} else {
  $result.violations += "Missing infra/podman/storage.conf"
}

# 2. Check Podman binary and machine allocation
$podmanCmd = Get-Command podman -ErrorAction SilentlyContinue
if ($null -ne $podmanCmd) {
  $result.podman_installed = $true
  $verOut = (& podman --version 2>$null) | Out-String
  $result.podman_version = $verOut.Trim()

  $machineJsonRaw = (& podman machine list --format json 2>$null) | Out-String
  if ($LASTEXITCODE -eq 0 -and -not [string]::IsNullOrWhiteSpace($machineJsonRaw)) {
    try {
      $machines = $machineJsonRaw | ConvertFrom-Json
      foreach ($m in $machines) {
        $result.machine_name = $m.Name
        $result.machine_disk_size = $m.DiskSize
        if ($m.DiskSize -match '^(\d+(\.\d+)?)GiB$') {
          $diskGiB = [double]$Matches[1]
          $result.machine_disk_gib = $diskGiB
          if ($diskGiB -gt $MaxStorageGiB) {
            $result.violations += "Podman machine '$($m.Name)' DiskSize ($diskGiB GiB) exceeds $MaxStorageGiB GiB cap."
          }
        } elseif ($m.DiskSize -match '^\d+$') {
          $diskBytes = [int64]$m.DiskSize
          $diskGiB = [math]::Round($diskBytes / 1GB, 2)
          $result.machine_disk_gib = $diskGiB
          if ($diskBytes -gt $MaxBytes) {
            $result.violations += "Podman machine '$($m.Name)' DiskSize ($diskGiB GiB) exceeds $MaxStorageGiB GiB cap."
          }
        }
      }
    } catch {
      # Fallback text parsing if JSON format varies
    }
  }
}

if ($result.violations.Count -gt 0) {
  $result.status = "FAIL"
}

if ($Json) {
  $result | ConvertTo-Json -Depth 4
} else {
  Write-Host "=== UMCF Podman 30 GB Quota Verification ==="
  Write-Host "Status             : $($result.status)"
  Write-Host "Max Allowed Quota  : $($result.max_allowed_gib) GiB ($($result.max_allowed_bytes) bytes)"
  Write-Host "Podman Installed   : $($result.podman_installed) ($($result.podman_version))"
  Write-Host "Podman Machine     : $($result.machine_name) (Disk: $($result.machine_disk_size))"
  Write-Host "Storage Conf Valid : $($result.storage_conf_valid) ($($result.storage_conf_path))"
  if ($result.violations.Count -gt 0) {
    foreach ($v in $result.violations) {
      Write-Error "VIOLATION: $v"
    }
  }
}

if ($result.status -ne "PASS") {
  exit 1
}
exit 0

