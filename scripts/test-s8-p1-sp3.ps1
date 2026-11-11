<#
.SYNOPSIS
  Executes classified five-tier verification tests for Subphase S8.P1.SP3:
  - Unit: OLS linear regression leak slope detector (catching 16 B/cycle leaks), host process FD/handle queries,
          and asynchronous quiescence barrier waiting for background drops.
  - Integration: 1,000-cycle accelerated CI soak test with multi-workload lifecycles, and abrupt disconnect storm.
  - Simulation: 10,000-cycle continuous soak test with periodic peer churn, cancellations, and OLS regression analysis.
  - Hardware-Required: Real host RAM, VRAM, and handle leak stability watchdog.
  - Manually-Verified: Svelte 5 Resource & Leak Diagnostics schema and flat trendline verification.
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
Write-Host "[1/5] Running [Unit] tests for OLS regression, host FD counter, and quiescence barrier..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $u1 = & cargo test -p soak-suite --test soak_tests test_ols_linear_regression_leak_slope_detector_catches_16_byte_per_cycle_leak 2>&1 | Out-String
  $pass1 = ($LASTEXITCODE -eq 0)
  $u2 = & cargo test -p soak-suite --test soak_tests test_fd_and_os_handle_counter_accuracy_windows_linux_macos 2>&1 | Out-String
  $pass2 = ($LASTEXITCODE -eq 0)
  $u3 = & cargo test -p soak-suite --test soak_tests test_quiescence_barrier_waits_for_background_drop_tasks 2>&1 | Out-String
  $pass3 = ($LASTEXITCODE -eq 0)

  $ErrorActionPreference = $prevEap

  if ($pass1 -and $pass2 -and $pass3) {
    $testResults.unit = "PASS"
    $evidenceDetails["unit"] = "OLS regression slope sensitivity (16B/cycle), OS handle counting, and quiescence barrier PASS"
    Write-Host "  -> [Unit] PASS" -ForegroundColor Green
  } else {
    Write-Host "  -> [Unit] FAIL`n$u1`n$u2`n$u3" -ForegroundColor Red
  }
} catch {
  Write-Host "  -> [Unit] ERROR: $_" -ForegroundColor Red
}

# 2. INTEGRATION TESTS
Write-Host "[2/5] Running [Integration] tests for 1,000-cycle soak gate and abrupt disconnect storm..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $i1 = & cargo test -p soak-suite --test soak_tests test_1000_cycle_ci_accelerated_multi_workload_soak_zero_leak_gate 2>&1 | Out-String
  $passI1 = ($LASTEXITCODE -eq 0)
  $i2 = & cargo test -p soak-suite --test soak_tests test_abrupt_client_disconnect_storm_zero_socket_or_stream_handle_leak 2>&1 | Out-String
  $passI2 = ($LASTEXITCODE -eq 0)

  $ErrorActionPreference = $prevEap

  if ($passI1 -and $passI2) {
    $testResults.integration = "PASS"
    $evidenceDetails["integration"] = "1,000-cycle accelerated soak test and abrupt disconnect storm with zero stream leaks PASS"
    Write-Host "  -> [Integration] PASS" -ForegroundColor Green
  } else {
    Write-Host "  -> [Integration] FAIL`n$i1`n$i2" -ForegroundColor Red
  }
} catch {
  Write-Host "  -> [Integration] ERROR: $_" -ForegroundColor Red
}

# 3. SIMULATION TESTS
Write-Host "[3/5] Running [Simulation] tests for 10,000-cycle continuous soak campaign..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $s1 = & cargo test -p soak-suite --test soak_tests test_10000_cycle_accelerated_soak_with_churn_and_cancellations 2>&1 | Out-String
  $passS1 = ($LASTEXITCODE -eq 0)

  $ErrorActionPreference = $prevEap

  if ($passS1) {
    $testResults.simulation = "PASS"
    $evidenceDetails["simulation"] = "10,000-cycle continuous soak test with peer churn and cancellations verifying flat regression slopes PASS"
    Write-Host "  -> [Simulation] PASS" -ForegroundColor Green
  } else {
    Write-Host "  -> [Simulation] FAIL`n$s1" -ForegroundColor Red
  }
} catch {
  Write-Host "  -> [Simulation] ERROR: $_" -ForegroundColor Red
}

# 4. HARDWARE-REQUIRED TESTS
Write-Host "[4/5] Running [Hardware-Required] tests for authentic host hardware memory and handle stability..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $h1 = & cargo test -p soak-suite --test soak_tests test_hardware_process_rss_and_handle_leak_stability 2>&1 | Out-String
  $passH1 = ($LASTEXITCODE -eq 0)

  $ErrorActionPreference = $prevEap

  if ($passH1) {
    $testResults.hardware_required = "PASS"
    $evidenceDetails["hardware_required"] = "Host physical RAM and process handle stability verified PASS"
    Write-Host "  -> [Hardware-Required] PASS" -ForegroundColor Green
  } else {
    Write-Host "  -> [Hardware-Required] FAIL`n$h1" -ForegroundColor Red
  }
} catch {
  Write-Host "  -> [Hardware-Required] ERROR: $_" -ForegroundColor Red
}

# 5. MANUALLY-VERIFIED TESTS
Write-Host "[5/5] Running [Manually-Verified] tests for Zero-Leak Diagnostics UI schema..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $m1 = & cargo test -p soak-suite --test soak_tests test_manual_resource_and_leak_diagnostics_ui_schema 2>&1 | Out-String
  $passM1 = ($LASTEXITCODE -eq 0)

  $ErrorActionPreference = $prevEap

  if ($passM1) {
    $testResults.manually_verified = "PASS"
    $evidenceDetails["manually_verified"] = "Zero-leak diagnostics UI schema and regression payload verified PASS"
    Write-Host "  -> [Manually-Verified] PASS" -ForegroundColor Green
  } else {
    Write-Host "  -> [Manually-Verified] FAIL`n$m1" -ForegroundColor Red
  }
} catch {
  Write-Host "  -> [Manually-Verified] ERROR: $_" -ForegroundColor Red
}

# SUMMARY
Write-Host "`n=== S8.P1.SP3 Soak Testing & Leak Profiling Five-Tier Verification Summary ===" -ForegroundColor Cyan
$allPassed = $true
foreach ($tier in $testResults.Keys) {
  $status = $testResults[$tier]
  $color = if ($status -eq "PASS") { "Green" } else { "Red"; $allPassed = $false }
  Write-Host "  $tier : $status" -ForegroundColor $color
}

if ($Json) {
  $out = [ordered]@{
    subphase = "S8.P1.SP3"
    status   = if ($allPassed) { "PASS" } else { "FAIL" }
    results  = $testResults
    details  = $evidenceDetails
  }
  $out | ConvertTo-Json -Depth 5
}

if (-not $allPassed) {
  exit 1
}
exit 0

