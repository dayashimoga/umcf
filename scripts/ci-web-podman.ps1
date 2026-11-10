<#
.SYNOPSIS
  scripts/ci-web-podman.ps1 — UMCF Web & Edge Control Plane CI Pipeline Runner.
  Subphase S1.P3.SP1
#>
[CmdletBinding()]
param()

$ErrorActionPreference = "Stop"

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$RepoRoot = (Resolve-Path (Join-Path $ScriptDir "..")).Path
Set-Location $RepoRoot

$sw = [System.Diagnostics.Stopwatch]::StartNew()

Write-Host "==============================================================================" -ForegroundColor Cyan
Write-Host " UMCF Web & Edge Control Plane CI Pipeline" -ForegroundColor Cyan
Write-Host "==============================================================================" -ForegroundColor Cyan

# Step 1: Pre-step Quota & Placeholder Guard
Write-Host "[Step 1/5] Running pre-build storage quota and placeholder check..."
& powershell -ExecutionPolicy Bypass -File (Join-Path $ScriptDir "check-no-placeholders.ps1")
if ($LASTEXITCODE -ne 0) { throw "Placeholder check failed" }
& powershell -ExecutionPolicy Bypass -File (Join-Path $ScriptDir "podman-quota-check.ps1")
if ($LASTEXITCODE -ne 0) { throw "Quota check failed" }

# Step 2: Web Console Lint & Unit Tests
Write-Host "[Step 2/5] Running @umcf/web_console lint and unit tests..."
& node ./apps/web_console/scripts/check.mjs
if ($LASTEXITCODE -ne 0) { throw "web_console check failed" }
& node --test ./apps/web_console/test/console.test.mjs
if ($LASTEXITCODE -ne 0) { throw "web_console test failed" }

# Step 3: Cloudflare Control Plane Unit Tests
Write-Host "[Step 3/5] Running @umcf/cf_control_plane unit tests..."
& node --test ./services/cf_control_plane/test/worker.test.mjs
if ($LASTEXITCODE -ne 0) { throw "cf_control_plane test failed" }

# Step 4: Web & Admin Dashboard Lint & Unit Tests
Write-Host "[Step 4/5] Running @umcf/web-dashboard lint and unit tests..."
& node ./apps/web-dashboard/scripts/check.mjs
if ($LASTEXITCODE -ne 0) { throw "web-dashboard check failed" }
& node --test ./apps/web-dashboard/test/dashboard.test.mjs
if ($LASTEXITCODE -ne 0) { throw "web-dashboard test failed" }

# Step 5: Post-step Quota Guard
Write-Host "[Step 5/5] Running post-build storage quota verification..."
& powershell -ExecutionPolicy Bypass -File (Join-Path $ScriptDir "podman-quota-check.ps1")
if ($LASTEXITCODE -ne 0) { throw "Post-build quota check failed" }

$sw.Stop()
$durationSec = [math]::Round($sw.Elapsed.TotalSeconds, 2)

Write-Host "==============================================================================" -ForegroundColor Green
Write-Host " UMCF Web CI Pipeline: ALL CHECKS PASSED in $($durationSec)s (< 60s target)" -ForegroundColor Green
Write-Host "==============================================================================" -ForegroundColor Green
