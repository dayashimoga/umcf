<#
.SYNOPSIS
  Executes classified tests for Subphase S4.P1.SP2:
  - Unit: ThermalBatteryGuard regime transitions (FullCapacity -> Throttled50 -> Paused -> EmergencyEvict) and temperature/battery hysteresis deadbands (3°C / 3%)
  - Integration: Simulated sensor trajectory stream with pause/resume broadcast and OS onTrimMemory critical memory reclamation (< 100 ms)
  - Simulation: Abrupt battery drop (25% -> 14%) and AC unplug fault injection (< 10 ms)
  - Hardware-Required: System clock and Podman storage quota compliance (< 29.0 GiB)
  - Manually-Verified: thermal_battery_hysteresis_trace.json audit
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
Write-Host "[1/5] Running [Unit] tests for ThermalBatteryGuard transitions and hysteresis deadbands..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $u1 = & cargo test -p umcf-guard --test guard_tests test_unit_thermal_battery_guard_regime_transitions 2>&1 | Out-String
  $p1 = ($LASTEXITCODE -eq 0)
  $u2 = & cargo test -p umcf-guard --test guard_tests test_unit_hysteresis_deadband_oscillation_prevention 2>&1 | Out-String
  $p2 = ($LASTEXITCODE -eq 0)
  $u3 = & cargo test -p umcf-guard --test guard_tests test_unit_battery_hysteresis_deadband 2>&1 | Out-String
  $p3 = ($LASTEXITCODE -eq 0)
  $pass = ($p1 -and $p2 -and $p3)

  $ErrorActionPreference = $prevEap

  if ($pass) {
    $testResults.unit = "PASS"
    $evidenceDetails["unit"] = "ThermalBatteryGuard 4-regime transitions, 3°C cooling deadband oscillation prevention, and 3% battery recharge hysteresis PASS"
    Write-Host "  -> [Unit] PASS" -ForegroundColor Green
  } else {
    Write-Host "  -> [Unit] FAIL`n$u1`n$u2`n$u3" -ForegroundColor Red
  }
} catch {
  Write-Host "  -> [Unit] ERROR: $_" -ForegroundColor Red
}

# 2. INTEGRATION TESTS
Write-Host "[2/5] Running [Integration] sensor trajectory stream and OS onTrimMemory handling..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $i1 = & cargo test -p umcf-guard --test guard_tests test_integration_sensor_stream_throttle_pause_resume -- --nocapture 2>&1 | Out-String
  $ip1 = ($LASTEXITCODE -eq 0)
  $i2 = & cargo test -p umcf-guard --test guard_tests test_integration_os_trim_memory_emergency_evict -- --nocapture 2>&1 | Out-String
  $ip2 = ($LASTEXITCODE -eq 0)
  $intPass = ($ip1 -and $ip2)

  $ErrorActionPreference = $prevEap

  if ($intPass) {
    $testResults.integration = "PASS"
    $evidenceDetails["integration"] = "Sensor stream throttle/pause/resume verified with < 50 ms transition latency; OS onTrimMemory(RunningCritical) handled in < 100 ms with EvacuateNode emission PASS"
    Write-Host "  -> [Integration] PASS" -ForegroundColor Green
  } else {
    Write-Host "  -> [Integration] FAIL`n$i1`n$i2" -ForegroundColor Red
  }
} catch {
  Write-Host "  -> [Integration] ERROR: $_" -ForegroundColor Red
}

# 3. SIMULATION / FAULT-INJECTION TESTS
Write-Host "[3/5] Running [Simulation] abrupt battery drop and AC unplug fault injection..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $simOut = & cargo test -p umcf-guard --test guard_tests test_simulation_abrupt_battery_drop_and_unplug_fault_injection 2>&1 | Out-String
  $simPass = ($LASTEXITCODE -eq 0)

  $ErrorActionPreference = $prevEap

  if ($simPass) {
    $testResults.simulation = "PASS"
    $evidenceDetails["simulation"] = "Abrupt battery drop (25% -> 14%) and AC unplug fault injection triggered immediate worker suspension and SuspendWorkload emission in < 10 ms PASS"
    Write-Host "  -> [Simulation] PASS" -ForegroundColor Green
  } else {
    Write-Host "  -> [Simulation] FAIL`n$simOut" -ForegroundColor Red
  }
} catch {
  Write-Host "  -> [Simulation] ERROR: $_" -ForegroundColor Red
}

# 4. HARDWARE-REQUIRED TESTS
Write-Host "[4/5] Running [Hardware-Required] system clock and Podman quota..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $hwOut = & cargo test -p umcf-guard --test guard_tests test_hw_required_guard_real_clock_and_podman_quota -- --nocapture 2>&1 | Out-String
  $hwPass = ($LASTEXITCODE -eq 0)

  $quotaOutput = & powershell -ExecutionPolicy Bypass -File (Join-Path $ScriptDir "podman-quota-check.ps1") 2>&1 | Out-String
  $quotaPass = ($LASTEXITCODE -eq 0)

  $ErrorActionPreference = $prevEap

  if ($hwPass -and $quotaPass) {
    $testResults.hardware_required = "PASS"
    $evidenceDetails["hardware_required"] = "Real hardware clock verified; Podman storage quota compliance PASS (< 29.0 GiB)"
    Write-Host "  -> [Hardware-Required] PASS" -ForegroundColor Green
  } else {
    Write-Host "  -> [Hardware-Required] FAIL (HW: $hwPass, Quota: $quotaPass)" -ForegroundColor Red
  }
} catch {
  Write-Host "  -> [Hardware-Required] ERROR: $_" -ForegroundColor Red
}

# 5. MANUALLY-VERIFIED TESTS
Write-Host "[5/5] Running [Manually-Verified] thermal_battery_hysteresis_trace.json audit..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $auditOut = & cargo test -p umcf-guard --test guard_tests test_manually_verified_thermal_battery_hysteresis_trace_audit 2>&1 | Out-String
  $auditPass = ($LASTEXITCODE -eq 0)

  $ErrorActionPreference = $prevEap

  $tracePath = Join-Path $RepoRoot "artifacts/evidence/stage-04/thermal_battery_hysteresis_trace.json"
  if ($auditPass -and (Test-Path $tracePath)) {
    $trace = Get-Content $tracePath -Raw | ConvertFrom-Json

    $count = $trace.Count
    $step4 = $trace[3] # 41.2°C -> PausedWaitCoolOrCharge
    $step5 = $trace[4] # 40.5°C -> still PausedWaitCoolOrCharge (no transition)

    $step4Ok = ($step4.current_regime -eq "PausedWaitCoolOrCharge" -and $step4.transition_occurred -eq $true)
    $step5Ok = ($step5.current_regime -eq "PausedWaitCoolOrCharge" -and $step5.transition_occurred -eq $false)

    if ($count -eq 10 -and $step4Ok -and $step5Ok) {
      $testResults.manually_verified = "PASS"
      $evidenceDetails["manually_verified"] = "thermal_battery_hysteresis_trace.json verified: 10 steps, pause transition at 41.2°C, hysteresis deadband confirmed at 40.5°C with zero flapping PASS"
      Write-Host "  -> [Manually-Verified] PASS" -ForegroundColor Green
    } else {
      Write-Host "  -> [Manually-Verified] FAIL (Trace invalid or hysteresis failed: step4=$step4Ok, step5=$step5Ok)" -ForegroundColor Red
    }
  } else {
    Write-Host "  -> [Manually-Verified] FAIL (Audit test failed or trace missing)`n$auditOut" -ForegroundColor Red
  }
} catch {
  Write-Host "  -> [Manually-Verified] ERROR: $_" -ForegroundColor Red
}

Write-Host "`nTest Summary for Subphase S4.P1.SP2:"
$allPassed = $true
foreach ($tier in $testResults.Keys) {
  $res = $testResults[$tier]
  $color = if ($res -eq "PASS") { "Green" } else { "Red" }
  Write-Host "  ${tier}: $res" -ForegroundColor $color
  if ($res -ne "PASS") {
    $allPassed = $false
  }
}

if ($Json) {
  $summary = [ordered]@{
    subphase = "S4.P1.SP2"
    timestamp_utc = (Get-Date -AsUTC -Format "o")
    all_passed = $allPassed
    results = $testResults
    details = $evidenceDetails
  }
  $summary | ConvertTo-Json -Depth 5
}

if (-not $allPassed) {
  exit 1
}
exit 0
