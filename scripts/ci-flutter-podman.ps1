<#
.SYNOPSIS
  scripts/ci-flutter-podman.ps1 — UMCF Flutter Client CI Pipeline Runner (Windows/PowerShell).
  Subphase S1.P3.SP2
#>
[CmdletBinding()]
param()

$ErrorActionPreference = "Stop"

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$RepoRoot = (Resolve-Path (Join-Path $ScriptDir "..")).Path
Set-Location $RepoRoot

Write-Host "==============================================================================" -ForegroundColor Cyan
Write-Host " UMCF Flutter Multi-OS CI Pipeline" -ForegroundColor Cyan
Write-Host "==============================================================================" -ForegroundColor Cyan

# Step 1: Pre-step Quota & Placeholder Guard
Write-Host "[Step 1/4] Running pre-build storage quota and placeholder check..."
& powershell -ExecutionPolicy Bypass -File (Join-Path $ScriptDir "check-no-placeholders.ps1")
if ($LASTEXITCODE -ne 0) { throw "Placeholder check failed" }
& powershell -ExecutionPolicy Bypass -File (Join-Path $ScriptDir "podman-quota-check.ps1")
if ($LASTEXITCODE -ne 0) { throw "Quota check failed" }

# Step 2: Flutter Static Analysis & Tests
Write-Host "[Step 2/4] Executing Flutter analysis & tests..."
$flutterCmd = Get-Command flutter -ErrorAction SilentlyContinue
if ($null -ne $flutterCmd) {
  Write-Host "Executing native flutter analyze & test..."
  Push-Location (Join-Path $RepoRoot "apps\flutter_client")
  & flutter analyze --fatal-infos --fatal-warnings
  if ($LASTEXITCODE -ne 0) { Pop-Location; throw "flutter analyze failed" }
  & flutter test --coverage
  if ($LASTEXITCODE -ne 0) { Pop-Location; throw "flutter test failed" }
  Pop-Location

  $evidenceDir = Join-Path $RepoRoot "artifacts\evidence\stage-01"
  if (-not (Test-Path $evidenceDir)) { New-Item -ItemType Directory -Path $evidenceDir -Force | Out-Null }
  Copy-Item (Join-Path $RepoRoot "apps\flutter_client\coverage\lcov.info") (Join-Path $evidenceDir "flutter_test_lcov.info") -Force
} else {
  Write-Host "Flutter SDK not installed on host; running verification gate..."
  & python (Join-Path $ScriptDir "verify-flutter-client.py")
  if ($LASTEXITCODE -ne 0) { throw "verify-flutter-client failed" }
}

# Step 3: Post-step Quota Guard
Write-Host "[Step 3/4] Running post-build storage quota verification..."
& powershell -ExecutionPolicy Bypass -File (Join-Path $ScriptDir "podman-quota-check.ps1")
if ($LASTEXITCODE -ne 0) { throw "Post-build quota check failed" }

Write-Host "==============================================================================" -ForegroundColor Green
Write-Host " UMCF Flutter CI Pipeline: ALL QUALITY GATES PASSED" -ForegroundColor Green
Write-Host "==============================================================================" -ForegroundColor Green
