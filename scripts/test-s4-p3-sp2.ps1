<#
.SYNOPSIS
  Executes classified tests for Subphase S4.P3.SP2:
  - Unit: PhiAccrualDetector sliding window math and CheckpointManager roundtrip/corruption detection
  - Integration: Checkpoint failover with 100% output parity and >= 80% wasted-work reduction
  - Simulation: Tail straggler speculative backup execution and idempotent single-winner commit barrier
  - Hardware-Required: Monotonic system clock and Podman storage quota compliance (< 29.0 GiB)
  - Manually-Verified: checkpoint_failover_trace.json audit verification
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
Write-Host "[1/5] Running [Unit] tests for PhiAccrualDetector and CheckpointManager..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $u1 = & cargo test -p umcf-worker --test worker_tests test_unit_s4_p3_sp2_ 2>&1 | Out-String
  $pass = ($LASTEXITCODE -eq 0)

  $ErrorActionPreference = $prevEap

  if ($pass) {
    $testResults.unit = "PASS"
    $evidenceDetails["unit"] = "PhiAccrualDetector sliding window variance math and CheckpointManager roundtrip/corruption detection PASS"
    Write-Host "  -> [Unit] PASS" -ForegroundColor Green
  } else {
    Write-Host "  -> [Unit] FAIL`n$u1" -ForegroundColor Red
  }
} catch {
  Write-Host "  -> [Unit] ERROR: $_" -ForegroundColor Red
}

# 2. INTEGRATION TESTS
Write-Host "[2/5] Running [Integration] checkpoint failover output parity and wasted work reduction..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $i1 = & cargo test -p umcf-worker --test worker_tests test_integration_s4_p3_sp2_ -- --nocapture 2>&1 | Out-String
  $intPass = ($LASTEXITCODE -eq 0)

  $ErrorActionPreference = $prevEap

  if ($intPass) {
    $testResults.integration = "PASS"
    $evidenceDetails["integration"] = "Checkpoint failover restores state from latest checkpoint with 100% output parity and >= 80% wasted work reduction PASS"
    Write-Host "  -> [Integration] PASS" -ForegroundColor Green
  } else {
    Write-Host "  -> [Integration] FAIL`n$i1" -ForegroundColor Red
  }
} catch {
  Write-Host "  -> [Integration] ERROR: $_" -ForegroundColor Red
}

# 3. SIMULATION TESTS
Write-Host "[3/5] Running [Simulation] straggler speculative retry and single-winner commit barrier..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $s1 = & cargo test -p umcf-worker --test worker_tests test_simulation_s4_p3_sp2_ -- --nocapture 2>&1 | Out-String
  $simPass = ($LASTEXITCODE -eq 0)

  $ErrorActionPreference = $prevEap

  if ($simPass) {
    $testResults.simulation = "PASS"
    $evidenceDetails["simulation"] = "Tail straggler detected, speculative backup launched, and idempotent commit barrier guaranteed single winner PASS"
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

  $hwTest = & cargo test -p umcf-worker --test worker_tests test_hardware_s4_p3_sp2_ 2>&1 | Out-String
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
Write-Host "[5/5] Running [Manually-Verified] checkpoint_failover_trace.json audit..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $mvTest = & cargo test -p umcf-worker --test worker_tests test_manually_verified_s4_p3_sp2_ 2>&1 | Out-String
  $mvPass = ($LASTEXITCODE -eq 0)

  $reportPath = Join-Path $RepoRoot "artifacts\evidence\stage-04\checkpoint_failover_trace.json"
  $reportExists = Test-Path $reportPath

  $ErrorActionPreference = $prevEap

  if ($mvPass -and $reportExists) {
    $reportData = Get-Content $reportPath -Raw | ConvertFrom-Json
    if ($reportData.lan_failure_detection_ms -le 1500.0 -and `
        $reportData.wan_failure_detection_ms -le 3500.0 -and `
        $reportData.wasted_work_reduction_percent -ge 80.0 -and `
        $reportData.output_hash_parity_verified -eq $true -and `
        $reportData.idempotent_commit_barrier_verified -eq $true) {
      $testResults.manually_verified = "PASS"
      $evidenceDetails["manually_verified"] = "checkpoint_failover_trace.json verified: LAN $($reportData.lan_failure_detection_ms) ms <= 1500 ms, WAN $($reportData.wan_failure_detection_ms) ms <= 3500 ms, work reduction $($reportData.wasted_work_reduction_percent)% >= 80% PASS"
      Write-Host "  -> [Manually-Verified] PASS" -ForegroundColor Green
    } else {
      Write-Host "  -> [Manually-Verified] FAIL: metrics failed thresholds" -ForegroundColor Red
    }
  } else {
    Write-Host "  -> [Manually-Verified] FAIL`n$mvTest" -ForegroundColor Red
  }
} catch {
  Write-Host "  -> [Manually-Verified] ERROR: $_" -ForegroundColor Red
}

Write-Host ""
Write-Host "Test Summary for Subphase S4.P3.SP2:"
$allPass = $true
foreach ($tier in $testResults.Keys) {
  $status = $testResults[$tier]
  $color = if ($status -eq "PASS") { "Green" } else { "Red" }
  Write-Host "  $tier`: $status" -ForegroundColor $color
  if ($status -ne "PASS") { $allPass = $false }
}

if ($Json) {
  $output = [ordered]@{
    subphase = "S4.P3.SP2"
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

