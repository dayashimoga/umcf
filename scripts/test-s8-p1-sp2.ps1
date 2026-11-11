<#
.SYNOPSIS
  Executes classified five-tier verification tests for Subphase S8.P1.SP2:
  - Unit: Asymmetric network partition matrix, deterministic PRNG seed reproducibility,
          and continuous invariant monitor synthetic split-brain detection.
  - Integration: Jepsen-style majority/minority split-brain and heal, simultaneous thermal critical & low battery
                 live task migration, mobile background 30s freeze lease expiry, and memory pressure cache eviction.
  - Simulation: 25-seed randomized combined Chaos Nemesis campaign matrix verifying all 5 invariants.
  - Hardware-Required: Real host CPU core and physical RAM governance watchdog.
  - Manually-Verified: Svelte 5 and Flutter Chaos Drawer / Fault Injection schema & telemetry.
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
Write-Host "[1/5] Running [Unit] tests for partition routing, deterministic PRNG, and invariant monitor..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $u1 = & cargo test -p umcf-chaos --test chaos_tests test_asymmetric_network_partition_routing_matrix 2>&1 | Out-String
  $pass1 = ($LASTEXITCODE -eq 0)
  $u2 = & cargo test -p umcf-chaos --test chaos_tests test_deterministic_prng_chaos_schedule_reproducibility 2>&1 | Out-String
  $pass2 = ($LASTEXITCODE -eq 0)
  $u3 = & cargo test -p umcf-chaos --test chaos_tests test_invariant_monitor_detects_synthetic_split_brain_divergence 2>&1 | Out-String
  $pass3 = ($LASTEXITCODE -eq 0)

  $ErrorActionPreference = $prevEap

  if ($pass1 -and $pass2 -and $pass3) {
    $testResults.unit = "PASS"
    $evidenceDetails["unit"] = "Routing matrix, deterministic PRNG seeds, and invariant monitor split-brain detection PASS"
    Write-Host "  -> [Unit] PASS" -ForegroundColor Green
  } else {
    Write-Host "  -> [Unit] FAIL`n$u1`n$u2`n$u3" -ForegroundColor Red
  }
} catch {
  Write-Host "  -> [Unit] ERROR: $_" -ForegroundColor Red
}

# 2. INTEGRATION TESTS
Write-Host "[2/5] Running [Integration] tests for split-brain heal, thermal/battery migration, mobile freeze, and memory trim..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $i1 = & cargo test -p umcf-chaos --test chaos_tests test_jepsen_style_majority_minority_split_brain_and_heal_under_load 2>&1 | Out-String
  $passI1 = ($LASTEXITCODE -eq 0)
  $i2 = & cargo test -p umcf-chaos --test chaos_tests test_simultaneous_thermal_critical_and_low_battery_live_task_migration 2>&1 | Out-String
  $passI2 = ($LASTEXITCODE -eq 0)
  $i3 = & cargo test -p umcf-chaos --test chaos_tests test_mobile_background_freeze_30s_lease_expiry_and_requeue 2>&1 | Out-String
  $passI3 = ($LASTEXITCODE -eq 0)
  $i4 = & cargo test -p umcf-chaos --test chaos_tests test_critical_memory_pressure_evicts_cache_and_migrates_worker_without_oom 2>&1 | Out-String
  $passI4 = ($LASTEXITCODE -eq 0)

  $ErrorActionPreference = $prevEap

  if ($passI1 -and $passI2 -and $passI3 -and $passI4) {
    $testResults.integration = "PASS"
    $evidenceDetails["integration"] = "Majority/minority split-brain, thermal/battery migration, 30s mobile freeze, and memory cache eviction PASS"
    Write-Host "  -> [Integration] PASS" -ForegroundColor Green
  } else {
    Write-Host "  -> [Integration] FAIL`n$i1`n$i2`n$i3`n$i4" -ForegroundColor Red
  }
} catch {
  Write-Host "  -> [Integration] ERROR: $_" -ForegroundColor Red
}

# 3. SIMULATION TESTS
Write-Host "[3/5] Running [Simulation] tests for 25-seed randomized Chaos Nemesis matrix..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $s1 = & cargo test -p umcf-chaos --test chaos_tests test_25_seed_randomized_chaos_campaign_matrix_verifies_all_invariants 2>&1 | Out-String
  $passS1 = ($LASTEXITCODE -eq 0)

  $ErrorActionPreference = $prevEap

  if ($passS1) {
    $testResults.simulation = "PASS"
    $evidenceDetails["simulation"] = "All 25 deterministic seeded chaos campaigns verified with 100% invariant compliance and bounded convergence PASS"
    Write-Host "  -> [Simulation] PASS" -ForegroundColor Green
  } else {
    Write-Host "  -> [Simulation] FAIL`n$s1" -ForegroundColor Red
  }
} catch {
  Write-Host "  -> [Simulation] ERROR: $_" -ForegroundColor Red
}

# 4. HARDWARE-REQUIRED TESTS
Write-Host "[4/5] Running [Hardware-Required] tests for authentic host hardware governance watchdog..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $h1 = & cargo test -p umcf-chaos --test chaos_tests test_hardware_link_and_thermal_governance_watchdog 2>&1 | Out-String
  $passH1 = ($LASTEXITCODE -eq 0)

  $ErrorActionPreference = $prevEap

  if ($passH1) {
    $testResults.hardware_required = "PASS"
    $evidenceDetails["hardware_required"] = "Real host CPU cores and physical RAM governance watchdog verified PASS"
    Write-Host "  -> [Hardware-Required] PASS" -ForegroundColor Green
  } else {
    Write-Host "  -> [Hardware-Required] FAIL`n$h1" -ForegroundColor Red
  }
} catch {
  Write-Host "  -> [Hardware-Required] ERROR: $_" -ForegroundColor Red
}

# 5. MANUALLY-VERIFIED TESTS
Write-Host "[5/5] Running [Manually-Verified] tests for Chaos Drawer schema and telemetry..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $m1 = & cargo test -p umcf-chaos --test chaos_tests test_manual_admin_chaos_drawer_schema_and_telemetry 2>&1 | Out-String
  $passM1 = ($LASTEXITCODE -eq 0)

  $ErrorActionPreference = $prevEap

  if ($passM1) {
    $testResults.manually_verified = "PASS"
    $evidenceDetails["manually_verified"] = "Chaos Drawer schema and telemetry report PASS"
    Write-Host "  -> [Manually-Verified] PASS" -ForegroundColor Green
  } else {
    Write-Host "  -> [Manually-Verified] FAIL`n$m1" -ForegroundColor Red
  }
} catch {
  Write-Host "  -> [Manually-Verified] ERROR: $_" -ForegroundColor Red
}

# SUMMARY
Write-Host "`n=== S8.P1.SP2 Chaos Engineering Five-Tier Verification Summary ===" -ForegroundColor Cyan
$allPassed = $true
foreach ($tier in $testResults.Keys) {
  $status = $testResults[$tier]
  $color = if ($status -eq "PASS") { "Green" } else { "Red"; $allPassed = $false }
  Write-Host "  $tier : $status" -ForegroundColor $color
}

if ($Json) {
  $out = [ordered]@{
    subphase = "S8.P1.SP2"
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
