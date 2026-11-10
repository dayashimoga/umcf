<#
.SYNOPSIS
  scripts/ci-core-podman.ps1 — UMCF Rust Core CI Pipeline Runner (Windows/PowerShell).
  Subphase S1.P3.SP1
#>
[CmdletBinding()]
param()

$ErrorActionPreference = "Stop"

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$RepoRoot = (Resolve-Path (Join-Path $ScriptDir "..")).Path
Set-Location $RepoRoot

Write-Host "==============================================================================" -ForegroundColor Cyan
Write-Host " UMCF Core CI Pipeline (Rust Workspace & Quality Gates)" -ForegroundColor Cyan
Write-Host "==============================================================================" -ForegroundColor Cyan

# Step 1: Pre-step Quota & Placeholder Guard
Write-Host "[Step 1/7] Running pre-build storage quota and placeholder check..."
& powershell -ExecutionPolicy Bypass -File (Join-Path $ScriptDir "check-no-placeholders.ps1")
if ($LASTEXITCODE -ne 0) { throw "Placeholder check failed" }
& powershell -ExecutionPolicy Bypass -File (Join-Path $ScriptDir "podman-quota-check.ps1")
if ($LASTEXITCODE -ne 0) { throw "Quota check failed" }

# Step 2: Code Formatting
Write-Host "[Step 2/7] Checking code formatting with cargo fmt..."
& cargo fmt --all -- --check
if ($LASTEXITCODE -ne 0) { throw "cargo fmt check failed" }

# Step 3: Clippy Static Analysis (Zero Warnings)
Write-Host "[Step 3/7] Running static analysis with cargo clippy..."
& cargo clippy --workspace --all-targets -- -D warnings
if ($LASTEXITCODE -ne 0) { throw "cargo clippy check failed" }

# Step 4: Workspace Test Suite
Write-Host "[Step 4/7] Running workspace test suite..."
& cargo test --workspace
if ($LASTEXITCODE -ne 0) { throw "cargo test failed" }

# Step 5: Coverage Generation
Write-Host "[Step 5/7] Generating workspace coverage report..."
$evidenceDir = Join-Path $RepoRoot "artifacts\evidence\stage-01"
if (-not (Test-Path $evidenceDir)) { New-Item -ItemType Directory -Path $evidenceDir -Force | Out-Null }
& python (Join-Path $ScriptDir "generate-stage01-coverage.py")
if ($LASTEXITCODE -ne 0) { throw "Coverage generation failed" }

# Step 6: Coverage Threshold Verification (>= 85.0%)
Write-Host "[Step 6/7] Enforcing coverage threshold gate (>= 85.0%)..."
$covReport = Join-Path $evidenceDir "coverage_stage01.json"
& python (Join-Path $ScriptDir "verify-coverage-threshold.py") --report $covReport --threshold 85.0
if ($LASTEXITCODE -ne 0) { throw "Coverage threshold verification failed" }

# Step 7: Post-step Quota Guard
Write-Host "[Step 7/7] Running post-build storage quota verification..."
& powershell -ExecutionPolicy Bypass -File (Join-Path $ScriptDir "podman-quota-check.ps1")
if ($LASTEXITCODE -ne 0) { throw "Post-build quota check failed" }

Write-Host "==============================================================================" -ForegroundColor Green
Write-Host " UMCF Core CI Pipeline: ALL QUALITY GATES PASSED (0 Warnings, >=85% Coverage)" -ForegroundColor Green
Write-Host "==============================================================================" -ForegroundColor Green
