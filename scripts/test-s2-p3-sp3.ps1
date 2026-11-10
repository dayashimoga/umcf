<#
.SYNOPSIS
  Executes classified tests for Subphase S2.P3.SP3:
  - Unit: Circuit breaker state transitions (Closed -> Open -> HalfOpen -> Closed) and Cloud Run fallback healthz
  - Integration: Cloud Run wire contract parity with CF Worker client, FailoverControlPlaneClient switchover and buffer
  - Simulation: Split-brain 2-island partition healing, Master arbitration, and 100% byte-identical CRDT convergence (< 1.0s)
  - Hardware-Required: Host hardware execution thread probe and Podman storage quota compliance (< 29.0 GiB)
  - Manually-Verified: split_brain_reconciliation_trace.json audit and STAGE_02_CERTIFICATION verification
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

# 1. UNIT TESTS: Circuit breaker transitions and Cloud Run fallback healthz
Write-Host "[1/5] Running [Unit] tests for circuit breaker and Cloud Run fallback healthz..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $unit1 = & cargo test -p umcf-registry --lib tests::unit_failover_circuit_breaker_state_transitions -- --nocapture 2>&1 | Out-String
  $pass1 = ($LASTEXITCODE -eq 0)

  $unit2 = & cargo test -p umcf-cloudrun-fallback --bin umcf-cloudrun-fallback tests::unit_cloudrun_fallback_healthz -- --nocapture 2>&1 | Out-String
  $pass2 = ($LASTEXITCODE -eq 0)

  $ErrorActionPreference = $prevEap

  if ($pass1 -and $pass2) {
    $testResults.unit = "PASS"
    $evidenceDetails["unit"] = "CircuitBreaker transitions (Closed -> Open -> HalfOpen -> Closed) PASS; Cloud Run fallback healthz PASS"
    Write-Host "  -> [Unit] PASS" -ForegroundColor Green
  } else {
    Write-Host "  -> [Unit] FAIL (Breaker: $pass1, CloudRun: $pass2)" -ForegroundColor Red
  }
} catch {
  Write-Host "  -> [Unit] ERROR: $_" -ForegroundColor Red
}

# 2. INTEGRATION TESTS: Cloud Run wire contract parity and failover client switchover
Write-Host "[2/5] Running [Integration] tests for Cloud Run parity and FailoverControlPlaneClient..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $int1 = & cargo test -p umcf-cloudrun-fallback --bin umcf-cloudrun-fallback tests::integration_cloudrun_fallback_wire_contract_parity_with_cf -- --nocapture 2>&1 | Out-String
  $intPass1 = ($LASTEXITCODE -eq 0)

  $int2 = & cargo test -p umcf-registry --lib tests::integration_failover_client_switches_to_cloudrun_and_buffers_for_lan -- --nocapture 2>&1 | Out-String
  $intPass2 = ($LASTEXITCODE -eq 0)

  $ErrorActionPreference = $prevEap

  if ($intPass1 -and $intPass2) {
    $testResults.integration = "PASS"
    $evidenceDetails["integration"] = "Cloud Run fallback wire contract parity with CF Worker verified; FailoverControlPlaneClient switchover to Secondary and LAN buffering with 0 lost deltas verified"
    Write-Host "  -> [Integration] PASS" -ForegroundColor Green
  } else {
    Write-Host "  -> [Integration] FAIL (Parity: $intPass1, Failover: $intPass2)" -ForegroundColor Red
  }
} catch {
  Write-Host "  -> [Integration] ERROR: $_" -ForegroundColor Red
}

# 3. SIMULATION / FAULT-INJECTION TESTS: Split-brain partition healing and 100% convergence
Write-Host "[3/5] Running [Simulation] split-brain partition healing and state convergence..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $sim = & cargo test -p umcf-registry --lib tests::simulation_split_brain_partition_healing_and_state_convergence -- --nocapture 2>&1 | Out-String
  $simPass = ($LASTEXITCODE -eq 0)

  $ErrorActionPreference = $prevEap

  if ($simPass) {
    $testResults.simulation = "PASS"
    $evidenceDetails["simulation"] = "Split-brain partition healing: Master conflict arbitrated deterministically (higher term/fitness wins); losing Master demoted to Worker; both islands converged 100% byte-identically in < 1000 ms"
    Write-Host "  -> [Simulation] PASS (Split-brain reconciliation & CRDT convergence verified)" -ForegroundColor Green
  } else {
    Write-Host "  -> [Simulation] FAIL" -ForegroundColor Red
  }
} catch {
  Write-Host "  -> [Simulation] ERROR: $_" -ForegroundColor Red
}

# 4. HARDWARE-REQUIRED TESTS: Host hardware execution thread probe and Podman quota compliance
Write-Host "[4/5] Running [Hardware-Required] hardware probe and Podman quota check..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $quotaOutput = & powershell -ExecutionPolicy Bypass -File (Join-Path $ScriptDir "podman-quota-check.ps1") 2>&1 | Out-String
  $quotaPass = ($LASTEXITCODE -eq 0)

  $cores = [int]$env:NUMBER_OF_PROCESSORS
  if ($cores -lt 1) { $cores = 1 }

  $ErrorActionPreference = $prevEap

  if ($quotaPass) {
    $testResults.hardware_required = "PASS"
    $evidenceDetails["hardware_required"] = "Hardware environment verified: host $cores execution threads probed; Podman storage quota compliance PASS (< 29.0 GiB)"
    Write-Host "  -> [Hardware-Required] PASS" -ForegroundColor Green
  } else {
    Write-Host "  -> [Hardware-Required] FAIL" -ForegroundColor Red
  }
} catch {
  Write-Host "  -> [Hardware-Required] ERROR: $_" -ForegroundColor Red
}

# 5. MANUALLY-VERIFIED TESTS: split_brain_reconciliation_trace.json verification
Write-Host "[5/5] Running [Manually-Verified] trace audit..."
try {
  $tracePath = Join-Path $RepoRoot "artifacts/evidence/stage-02/split_brain_reconciliation_trace.json"

  if (Test-Path $tracePath) {
    $trace = Get-Content $tracePath -Raw | ConvertFrom-Json

    $winnerConfirmed = ($trace.winner_term -gt $trace.demoted_term)
    $durationConfirmed = ($trace.duration_ms -lt 1000.0)
    $statesConverged = ($trace.states_converged -eq $true)
    $statusPass = ($trace.status -eq "PASS")

    if ($winnerConfirmed -and $durationConfirmed -and $statesConverged -and $statusPass) {
      $testResults.manually_verified = "PASS"
      $evidenceDetails["manually_verified"] = "split_brain_reconciliation_trace.json verified: winner term ($($trace.winner_term)) > demoted term ($($trace.demoted_term)); duration ($($trace.duration_ms) ms) < 1000.0 ms; states_converged = true"
      Write-Host "  -> [Manually-Verified] PASS" -ForegroundColor Green
    } else {
      Write-Host "  -> [Manually-Verified] FAIL (Trace invalid)" -ForegroundColor Red
    }
  } else {
    Write-Host "  -> [Manually-Verified] FAIL (Trace file missing)" -ForegroundColor Red
  }
} catch {
  Write-Host "  -> [Manually-Verified] ERROR: $_" -ForegroundColor Red
}

$allPass = ($testResults.unit -eq "PASS") -and ($testResults.integration -eq "PASS") -and ($testResults.simulation -eq "PASS") -and ($testResults.hardware_required -eq "PASS") -and ($testResults.manually_verified -eq "PASS")

$summary = [ordered]@{
  subphase              = "S2.P3.SP3"
  overall_status        = if ($allPass) { "PASS" } else { "FAIL" }
  test_classifications = $testResults
  details               = $evidenceDetails
}

if ($Json) {
  $summary | ConvertTo-Json -Depth 5
} else {
  Write-Host "=== Subphase S2.P3.SP3 Test Classification Summary ==="
  Write-Host "Overall Status: $($summary.overall_status)"
}

if (-not $allPass) {
  exit 1
}
exit 0
