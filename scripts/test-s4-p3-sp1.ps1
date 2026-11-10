<#
.SYNOPSIS
  Executes classified tests for Subphase S4.P3.SP1:
  - Unit: ArtifactStager BLAKE3 verification, EphemeralScratchDir lifecycle, Wasm fuel exhaustion and memory bounds
  - Integration: Wasm map-reduce pipeline with ExecutionReceipt and authorized native kernel execution
  - Simulation: All 5 sandbox escape vectors (traversal, absolute root, infinite loop, memory bomb, unauthorized native kernel) trapped < 20 ms
  - Hardware-Required: Monotonic system clock and Podman storage quota compliance (< 29.0 GiB)
  - Manually-Verified: sandbox_escape_audit.json audit verification
#>
param(
  [switch]$Json
)

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$RepoRoot = (Resolve-Path (Join-Path $ScriptDir "..")).Path

$testResults = [ordered]@{
  unit              = "FAIL"
  integration       = "FAIL"
  simulation        = "FAIL"
  hardware_required = "FAIL"
  manually_verified = "FAIL"
}

$evidenceDetails = [ordered]@{}

# 1. UNIT TESTS
Write-Host "[1/5] Running [Unit] tests for ArtifactStager, EphemeralScratchDir, and Wasm bounds..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $u1 = & cargo test -p umcf-worker --test worker_tests test_unit_ 2>&1 | Out-String
  $pass = ($LASTEXITCODE -eq 0)

  $ErrorActionPreference = $prevEap

  if ($pass) {
    $testResults.unit = "PASS"
    $evidenceDetails["unit"] = "ArtifactStager BLAKE3 verification, EphemeralScratchDir lifecycle, and Wasm fuel/memory/preemption bounds PASS"
    Write-Host "  -> [Unit] PASS" -ForegroundColor Green
  } else {
    Write-Host "  -> [Unit] FAIL`n$u1" -ForegroundColor Red
  }
} catch {
  Write-Host "  -> [Unit] ERROR: $_" -ForegroundColor Red
}

# 2. INTEGRATION TESTS
Write-Host "[2/5] Running [Integration] Wasm map-reduce transform and authorized native kernel execution..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $i1 = & cargo test -p umcf-worker --test worker_tests test_integration_ -- --nocapture 2>&1 | Out-String
  $intPass = ($LASTEXITCODE -eq 0)

  $ErrorActionPreference = $prevEap

  if ($intPass) {
    $testResults.integration = "PASS"
    $evidenceDetails["integration"] = "Wasm map-reduce transform sealed with ExecutionReceipt and authorized native kernel execution PASS"
    Write-Host "  -> [Integration] PASS" -ForegroundColor Green
  } else {
    Write-Host "  -> [Integration] FAIL`n$i1" -ForegroundColor Red
  }
} catch {
  Write-Host "  -> [Integration] ERROR: $_" -ForegroundColor Red
}

# 3. SIMULATION TESTS
Write-Host "[3/5] Running [Simulation] sandbox escape vector audit and unauthorized native kernel rejection..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $s1 = & cargo test -p umcf-worker --test worker_tests test_simulation_ -- --nocapture 2>&1 | Out-String
  $simPass = ($LASTEXITCODE -eq 0)

  $ErrorActionPreference = $prevEap

  if ($simPass) {
    $testResults.simulation = "PASS"
    $evidenceDetails["simulation"] = "All 5 sandbox escape vectors trapped cleanly within < 20 ms and unauthorized native kernel rejected with E4204 PASS"
    Write-Host "  -> [Simulation] PASS" -ForegroundColor Green
  } else {
    Write-Host "  -> [Simulation] FAIL`n$s1" -ForegroundColor Red
  }
} catch {
  Write-Host "  -> [Simulation] ERROR: $_" -ForegroundColor Red
}

# 4. HARDWARE-REQUIRED TESTS
Write-Host "[4/5] Running [Hardware-Required] monotonic timer and Podman quota..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $hwTest = & cargo test -p umcf-worker --test worker_tests test_hardware_ 2>&1 | Out-String
  $hwPass = ($LASTEXITCODE -eq 0)

  $quotaScript = Join-Path $RepoRoot "scripts\podman-quota-check.ps1"
  $quotaOut = & powershell -ExecutionPolicy Bypass -File $quotaScript -Json 2>&1 | Out-String
  $quotaPass = ($LASTEXITCODE -eq 0)

  $ErrorActionPreference = $prevEap

  if ($hwPass -and $quotaPass) {
    $testResults.hardware_required = "PASS"
    $evidenceDetails["hardware_required"] = "Monotonic timer and Podman storage quota compliance verified PASS"
    Write-Host "  -> [Hardware-Required] PASS" -ForegroundColor Green
  } else {
    Write-Host "  -> [Hardware-Required] FAIL`n$hwTest`n$quotaOut" -ForegroundColor Red
  }
} catch {
  Write-Host "  -> [Hardware-Required] ERROR: $_" -ForegroundColor Red
}

# 5. MANUALLY-VERIFIED TESTS
Write-Host "[5/5] Running [Manually-Verified] sandbox_escape_audit.json audit..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $mvTest = & cargo test -p umcf-worker --test worker_tests test_manually_verified_ 2>&1 | Out-String
  $mvPass = ($LASTEXITCODE -eq 0)

  $reportPath = Join-Path $RepoRoot "artifacts\evidence\stage-04\sandbox_escape_audit.json"
  $reportExists = Test-Path $reportPath

  $ErrorActionPreference = $prevEap

  if ($mvPass -and $reportExists) {
    $reportData = Get-Content $reportPath -Raw | ConvertFrom-Json
    if ($reportData.all_escapes_trapped -eq $true -and $reportData.max_trap_latency_ms -le 20.0) {
      $testResults.manually_verified = "PASS"
      $evidenceDetails["manually_verified"] = "sandbox_escape_audit.json verified: all escapes trapped, max latency $($reportData.max_trap_latency_ms) ms <= 20.0 ms PASS"
      Write-Host "  -> [Manually-Verified] PASS" -ForegroundColor Green
    } else {
      Write-Host "  -> [Manually-Verified] FAIL: invalid metrics or max trap latency > 20 ms" -ForegroundColor Red
    }
  } else {
    Write-Host "  -> [Manually-Verified] FAIL`n$mvTest" -ForegroundColor Red
  }
} catch {
  Write-Host "  -> [Manually-Verified] ERROR: $_" -ForegroundColor Red
}

Write-Host ""
Write-Host "Test Summary for Subphase S4.P3.SP1:"
$allPass = $true
foreach ($tier in $testResults.Keys) {
  $status = $testResults[$tier]
  $color = if ($status -eq "PASS") { "Green" } else { "Red" }
  Write-Host "  $tier`: $status" -ForegroundColor $color
  if ($status -ne "PASS") { $allPass = $false }
}

if ($Json) {
  $output = [ordered]@{
    subphase = "S4.P3.SP1"
    all_pass = $allPass
    tests    = $testResults
    details  = $evidenceDetails
  }
  $output | ConvertTo-Json -Depth 4
}

if ($allPass) {
  exit 0
} else {
  exit 1
}
