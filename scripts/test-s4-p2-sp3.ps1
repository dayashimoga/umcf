<#
.SYNOPSIS
  Executes classified tests for Subphase S4.P2.SP3:
  - Unit: Deficit Round-Robin quantum fairness, anti-starvation aging promotion, and admission rejection thresholds
  - Integration: InteractiveRealtime jobs bypass 50 queued batch jobs within strictly < 15.0 ms
  - Simulation: 20 concurrent gang-scheduled jobs across 4 nodes with zero deadlocks over 1,000 runs, and worker backpressure saturation
  - Hardware-Required: Monotonic system clock and Podman storage quota compliance (< 29.0 GiB)
  - Manually-Verified: queue_fairness_and_backpressure.json audit
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
Write-Host "[1/5] Running [Unit] tests for DRR fairness, priority aging, and admission control rejection..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $u1 = & cargo test -p umcf-scheduler --test scheduler_tests test_s4_p2_sp3_unit_drr_fairness_aging_and_admission_rejection 2>&1 | Out-String
  $pass = ($LASTEXITCODE -eq 0)

  $ErrorActionPreference = $prevEap

  if ($pass) {
    $testResults.unit = "PASS"
    $evidenceDetails["unit"] = "DRR quantum fairness, anti-starvation priority aging promotion, and admission control rejection thresholds (E4101) PASS"
    Write-Host "  -> [Unit] PASS" -ForegroundColor Green
  } else {
    Write-Host "  -> [Unit] FAIL`n$u1" -ForegroundColor Red
  }
} catch {
  Write-Host "  -> [Unit] ERROR: $_" -ForegroundColor Red
}

# 2. INTEGRATION TESTS
Write-Host "[2/5] Running [Integration] InteractiveRealtime bypass of 50 batch jobs strictly < 15 ms..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $i1 = & cargo test -p umcf-scheduler --test scheduler_tests test_s4_p2_sp3_integration_interactive_bypass_under_15ms -- --nocapture 2>&1 | Out-String
  $intPass = ($LASTEXITCODE -eq 0)

  $ErrorActionPreference = $prevEap

  if ($intPass) {
    $testResults.integration = "PASS"
    $evidenceDetails["integration"] = "InteractiveRealtime jobs bypassed 50 queued StandardBatch jobs and dispatched within < 15.0 ms PASS"
    Write-Host "  -> [Integration] PASS" -ForegroundColor Green
  } else {
    Write-Host "  -> [Integration] FAIL`n$i1" -ForegroundColor Red
  }
} catch {
  Write-Host "  -> [Integration] ERROR: $_" -ForegroundColor Red
}

# 3. SIMULATION TESTS
Write-Host "[3/5] Running [Simulation] deadlock-free gang scheduling (1,000 runs) and worker backpressure..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $s1 = & cargo test -p umcf-scheduler --test scheduler_tests test_s4_p2_sp3_simulation_deadlock_free_gang_scheduling -- --nocapture 2>&1 | Out-String
  $sp1 = ($LASTEXITCODE -eq 0)
  $s2 = & cargo test -p umcf-scheduler --test scheduler_tests test_s4_p2_sp3_simulation_worker_backpressure_saturation -- --nocapture 2>&1 | Out-String
  $sp2 = ($LASTEXITCODE -eq 0)
  $simPass = ($sp1 -and $sp2)

  $ErrorActionPreference = $prevEap

  if ($simPass) {
    $testResults.simulation = "PASS"
    $evidenceDetails["simulation"] = "Deadlock-free gang scheduling verified over 1,000 concurrent contending runs and worker backpressure saturation verified PASS"
    Write-Host "  -> [Simulation] PASS" -ForegroundColor Green
  } else {
    Write-Host "  -> [Simulation] FAIL`n$s1`n$s2" -ForegroundColor Red
  }
} catch {
  Write-Host "  -> [Simulation] ERROR: $_" -ForegroundColor Red
}

# 4. HARDWARE-REQUIRED TESTS
Write-Host "[4/5] Running [Hardware-Required] monotonic timer and Podman quota..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $hwTest = & cargo test -p umcf-scheduler --test scheduler_tests test_s4_p2_sp3_hardware_monotonic_timer_and_quota 2>&1 | Out-String
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
Write-Host "[5/5] Running [Manually-Verified] queue_fairness_and_backpressure.json audit..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $mvTest = & cargo test -p umcf-scheduler --test scheduler_tests test_s4_p2_sp3_manually_verified_queue_fairness_json_audit 2>&1 | Out-String
  $mvPass = ($LASTEXITCODE -eq 0)

  $reportPath = Join-Path $RepoRoot "artifacts\evidence\stage-04\queue_fairness_and_backpressure.json"
  $reportExists = Test-Path $reportPath

  $ErrorActionPreference = $prevEap

  if ($mvPass -and $reportExists) {
    $reportData = Get-Content $reportPath -Raw | ConvertFrom-Json
    if ($reportData.total_enqueued -gt 0 -and $reportData.p99_wait_time_ms -le 15.0) {
      $testResults.manually_verified = "PASS"
      $evidenceDetails["manually_verified"] = "queue_fairness_and_backpressure.json verified: total enqueued $($reportData.total_enqueued), p99 wait time $($reportData.p99_wait_time_ms) ms <= 15.0 ms PASS"
      Write-Host "  -> [Manually-Verified] PASS" -ForegroundColor Green
    } else {
      Write-Host "  -> [Manually-Verified] FAIL: invalid metrics or p99 wait time > 15 ms" -ForegroundColor Red
    }
  } else {
    Write-Host "  -> [Manually-Verified] FAIL`n$mvTest" -ForegroundColor Red
  }
} catch {
  Write-Host "  -> [Manually-Verified] ERROR: $_" -ForegroundColor Red
}

Write-Host ""
Write-Host "Test Summary for Subphase S4.P2.SP3:"
$allPass = $true
foreach ($tier in $testResults.Keys) {
  $status = $testResults[$tier]
  $color = if ($status -eq "PASS") { "Green" } else { "Red" }
  Write-Host "  $tier`: $status" -ForegroundColor $color
  if ($status -ne "PASS") { $allPass = $false }
}

if ($Json) {
  $output = [ordered]@{
    subphase = "S4.P2.SP3"
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

