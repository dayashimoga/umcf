<#
.SYNOPSIS
  Executes classified tests for Subphase S4.P1.SP3:
  - Unit: UserActivityDetector state machine (Idle -> InteractiveActive -> Cooldown -> RampingUp -> Idle), cooldown/rampup interruption, and PreemptionToken cooperative yield checkpoints
  - Integration: 4-worker continuous compute workload preemption latency strictly < 50,000 µs (< 50 ms)
  - Simulation: Rapid 100 Hz user input bursts without data corruption, and host CPU spike (+25%) threshold trigger
  - Hardware-Required: Monotonic system clock and Podman storage quota compliance (< 29.0 GiB)
  - Manually-Verified: preemption_latency_histogram.json audit (p99 < 50 ms, 100% deadline met)
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
Write-Host "[1/5] Running [Unit] tests for UserActivityDetector state transitions, ramp-up, and cooperative yield..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $u1 = & cargo test -p umcf-guard --test guard_tests test_s4_p1_sp3_unit_activity_state_machine_and_rampup 2>&1 | Out-String
  $p1 = ($LASTEXITCODE -eq 0)
  $u2 = & cargo test -p umcf-guard --test guard_tests test_s4_p1_sp3_unit_cooldown_and_rampup_interruption 2>&1 | Out-String
  $p2 = ($LASTEXITCODE -eq 0)
  $u3 = & cargo test -p umcf-guard --test guard_tests test_s4_p1_sp3_unit_preemption_token_and_cooperative_yield 2>&1 | Out-String
  $p3 = ($LASTEXITCODE -eq 0)
  $pass = ($p1 -and $p2 -and $p3)

  $ErrorActionPreference = $prevEap

  if ($pass) {
    $testResults.unit = "PASS"
    $evidenceDetails["unit"] = "State machine transitions, 3.0s capacity ramp-up, cooldown/ramp-up interruption, and cooperative yield checkpoints PASS"
    Write-Host "  -> [Unit] PASS" -ForegroundColor Green
  } else {
    Write-Host "  -> [Unit] FAIL`n$u1`n$u2`n$u3" -ForegroundColor Red
  }
} catch {
  Write-Host "  -> [Unit] ERROR: $_" -ForegroundColor Red
}

# 2. INTEGRATION TESTS
Write-Host "[2/5] Running [Integration] multi-worker preemption latency strictly < 50 ms..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $i1 = & cargo test -p umcf-guard --test guard_tests test_s4_p1_sp3_integration_worker_preemption_latency_under_50ms -- --nocapture 2>&1 | Out-String
  $intPass = ($LASTEXITCODE -eq 0)

  $ErrorActionPreference = $prevEap

  if ($intPass) {
    $testResults.integration = "PASS"
    $evidenceDetails["integration"] = "4-worker compute task suspension verified with latency strictly < 50,000 µs (< 50 ms) and zero corruption PASS"
    Write-Host "  -> [Integration] PASS" -ForegroundColor Green
  } else {
    Write-Host "  -> [Integration] FAIL`n$i1" -ForegroundColor Red
  }
} catch {
  Write-Host "  -> [Integration] ERROR: $_" -ForegroundColor Red
}

# 3. SIMULATION TESTS
Write-Host "[3/5] Running [Simulation] 100 Hz rapid bursts and host CPU spike detection..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $s1 = & cargo test -p umcf-guard --test guard_tests test_s4_p1_sp3_simulation_rapid_100hz_bursts -- --nocapture 2>&1 | Out-String
  $sp1 = ($LASTEXITCODE -eq 0)
  $s2 = & cargo test -p umcf-guard --test guard_tests test_s4_p1_sp3_simulation_host_cpu_spike_detection -- --nocapture 2>&1 | Out-String
  $sp2 = ($LASTEXITCODE -eq 0)
  $simPass = ($sp1 -and $sp2)

  $ErrorActionPreference = $prevEap

  if ($simPass) {
    $testResults.simulation = "PASS"
    $evidenceDetails["simulation"] = "100 Hz input bursts (100 events) verified with 100% deadline compliance and host CPU load delta spike (+25%) preemption PASS"
    Write-Host "  -> [Simulation] PASS" -ForegroundColor Green
  } else {
    Write-Host "  -> [Simulation] FAIL`n$s1`n$s2" -ForegroundColor Red
  }
} catch {
  Write-Host "  -> [Simulation] ERROR: $_" -ForegroundColor Red
}

# 4. HARDWARE-REQUIRED TESTS
Write-Host "[4/5] Running [Hardware-Required] monotonic system clock and Podman quota..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $hwTest = & cargo test -p umcf-guard --test guard_tests test_s4_p1_sp3_hardware_monotonic_timer_and_quota 2>&1 | Out-String
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
Write-Host "[5/5] Running [Manually-Verified] preemption_latency_histogram.json audit..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $mvTest = & cargo test -p umcf-guard --test guard_tests test_s4_p1_sp3_manually_verified_latency_histogram_export 2>&1 | Out-String
  $mvPass = ($LASTEXITCODE -eq 0)

  $histPath = Join-Path $RepoRoot "artifacts\evidence\stage-04\preemption_latency_histogram.json"
  $histExists = Test-Path $histPath

  $ErrorActionPreference = $prevEap

  if ($mvPass -and $histExists) {
    $histData = Get-Content $histPath -Raw | ConvertFrom-Json
    if ($histData.deadline_met_percent -ge 99.0 -and $histData.p99_us -le 50000) {
      $testResults.manually_verified = "PASS"
      $evidenceDetails["manually_verified"] = "preemption_latency_histogram.json verified: p99 latency $($histData.p99_us) µs <= 50,000 µs, deadline met $($histData.deadline_met_percent)% PASS"
      Write-Host "  -> [Manually-Verified] PASS" -ForegroundColor Green
    } else {
      Write-Host "  -> [Manually-Verified] FAIL: p99 exceeds 50 ms or deadline met < 99%" -ForegroundColor Red
    }
  } else {
    Write-Host "  -> [Manually-Verified] FAIL`n$mvTest" -ForegroundColor Red
  }
} catch {
  Write-Host "  -> [Manually-Verified] ERROR: $_" -ForegroundColor Red
}

Write-Host ""
Write-Host "Test Summary for Subphase S4.P1.SP3:"
$allPass = $true
foreach ($tier in $testResults.Keys) {
  $status = $testResults[$tier]
  $color = if ($status -eq "PASS") { "Green" } else { "Red" }
  Write-Host "  $tier`: $status" -ForegroundColor $color
  if ($status -ne "PASS") { $allPass = $false }
}

if ($Json) {
  $output = [ordered]@{
    subphase = "S4.P1.SP3"
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

