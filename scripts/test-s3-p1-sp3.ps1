<#
.SYNOPSIS
  Executes classified tests for Subphase S3.P1.SP3:
  - Unit: TrafficClass priority mapping, DSCP byte values, and BandwidthShaper token accounting (±3.5% accuracy)
  - Integration: QoS under 100% BackgroundBulk load, 100 Hz CriticalControl pings, zero drops, and p99 latency < 5.0 ms in release
  - Simulation: Dynamic BandwidthShaper throttling convergence (< 200 ms)
  - Hardware-Required: Network interface probe and Podman storage quota compliance (< 29.0 GiB)
  - Manually-Verified: OpenMetrics telemetry exposition and qos_latency_under_load.json audit
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
Write-Host "[1/5] Running [Unit] tests for TrafficClass priority mapping, DSCP values, and BandwidthShaper token accounting..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $unitOut = & cargo test -p umcf-net -j 2 --lib -- test_traffic_class_priorities test_bandwidth_shaper_token_accounting 2>&1 | Out-String
  $pass = ($LASTEXITCODE -eq 0)

  $ErrorActionPreference = $prevEap

  if ($pass) {
    $testResults.unit = "PASS"
    $evidenceDetails["unit"] = "TrafficClass 4-tier priorities and DSCP/TOS byte mapping PASS; BandwidthShaper token accounting & accuracy (<= 3.5%) PASS"
    Write-Host "  -> [Unit] PASS" -ForegroundColor Green
  } else {
    Write-Host "  -> [Unit] FAIL`n$unitOut" -ForegroundColor Red
  }
} catch {
  Write-Host "  -> [Unit] ERROR: $_" -ForegroundColor Red
}

# 2. INTEGRATION TESTS
Write-Host "[2/5] Running [Integration] QoS priority scheduling under 100% BackgroundBulk load (p99 latency < 5.0 ms, zero drops)..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $intOut = & cargo test -p umcf-net -j 2 --release --lib tests::test_qos_under_bulk_load_latency_and_zero_drops -- --nocapture 2>&1 | Out-String
  $intPass = ($LASTEXITCODE -eq 0)

  $ErrorActionPreference = $prevEap

  if ($intPass) {
    $testResults.integration = "PASS"
    $evidenceDetails["integration"] = "CriticalControl heartbeats maintained p99 latency < 5.0 ms under 100% BackgroundBulk saturation with 0 dropped pings"
    Write-Host "  -> [Integration] PASS" -ForegroundColor Green
  } else {
    Write-Host "  -> [Integration] FAIL`n$intOut" -ForegroundColor Red
  }
} catch {
  Write-Host "  -> [Integration] ERROR: $_" -ForegroundColor Red
}

# 3. SIMULATION / FAULT-INJECTION TESTS
Write-Host "[3/5] Running [Simulation] dynamic bandwidth throttling convergence (< 200 ms)..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $simOut = & cargo test -p umcf-net -j 2 --lib -- test_bandwidth_shaper_dynamic_throttling_convergence 2>&1 | Out-String
  $simPass = ($LASTEXITCODE -eq 0)

  $ErrorActionPreference = $prevEap

  if ($simPass) {
    $testResults.simulation = "PASS"
    $evidenceDetails["simulation"] = "Dynamic rate limit reduction from 100 Mbps to 10 Mbps converged in < 200 ms"
    Write-Host "  -> [Simulation] PASS" -ForegroundColor Green
  } else {
    Write-Host "  -> [Simulation] FAIL`n$simOut" -ForegroundColor Red
  }
} catch {
  Write-Host "  -> [Simulation] ERROR: $_" -ForegroundColor Red
}

# 4. HARDWARE-REQUIRED TESTS
Write-Host "[4/5] Running [Hardware-Required] network interface probe and Podman quota check..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $hwOut = & cargo test -p umcf-net -j 2 --lib tests::test_hardware_network_interfaces_probe -- --nocapture 2>&1 | Out-String
  $hwPass = ($LASTEXITCODE -eq 0)

  $quotaOutput = & powershell -ExecutionPolicy Bypass -File (Join-Path $ScriptDir "podman-quota-check.ps1") 2>&1 | Out-String
  $quotaPass = ($LASTEXITCODE -eq 0)

  $ErrorActionPreference = $prevEap

  if ($hwPass -and $quotaPass) {
    $testResults.hardware_required = "PASS"
    $evidenceDetails["hardware_required"] = "Loopback UDP socket verified; Podman storage quota compliance PASS (< 29.0 GiB)"
    Write-Host "  -> [Hardware-Required] PASS" -ForegroundColor Green
  } else {
    Write-Host "  -> [Hardware-Required] FAIL (HW: $hwPass, Quota: $quotaPass)" -ForegroundColor Red
  }
} catch {
  Write-Host "  -> [Hardware-Required] ERROR: $_" -ForegroundColor Red
}

# 5. MANUALLY-VERIFIED TESTS
Write-Host "[5/5] Running [Manually-Verified] OpenMetrics telemetry exposition and qos_latency_under_load.json audit..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $auditOut = & cargo test -p umcf-net -j 2 --lib -- test_qos_openmetrics test_qos_latency_under_load_manifest_audit 2>&1 | Out-String
  $auditPass = ($LASTEXITCODE -eq 0)

  $ErrorActionPreference = $prevEap

  $qosPath = Join-Path $RepoRoot "artifacts/evidence/stage-03/qos_latency_under_load.json"
  if ($auditPass -and (Test-Path $qosPath)) {
    $qos = Get-Content $qosPath -Raw | ConvertFrom-Json

    $latencyConfirmed = ($qos.control_latency_p99_ms -lt 5.0)
    $dropsConfirmed = ($qos.control_pings_dropped -eq 0)
    $statusPass = ($qos.status -eq "PASS")

    if ($latencyConfirmed -and $dropsConfirmed -and $statusPass) {
      $testResults.manually_verified = "PASS"
      $evidenceDetails["manually_verified"] = "OpenMetrics exposition verified; qos_latency_under_load.json verified: p99 latency ($($qos.control_latency_p99_ms) ms < 5.0 ms), drops = 0, status = PASS"
      Write-Host "  -> [Manually-Verified] PASS" -ForegroundColor Green
    } else {
      Write-Host "  -> [Manually-Verified] FAIL (QoS manifest values invalid: latency=$($qos.control_latency_p99_ms), drops=$($qos.control_pings_dropped))" -ForegroundColor Red
    }
  } else {
    Write-Host "  -> [Manually-Verified] FAIL (Audit failed or qos_latency_under_load.json missing)`n$auditOut" -ForegroundColor Red
  }
} catch {
  Write-Host "  -> [Manually-Verified] ERROR: $_" -ForegroundColor Red
}

$allPass = ($testResults.unit -eq "PASS") -and ($testResults.integration -eq "PASS") -and ($testResults.simulation -eq "PASS") -and ($testResults.hardware_required -eq "PASS") -and ($testResults.manually_verified -eq "PASS")

$summary = [ordered]@{
  subphase              = "S3.P1.SP3"
  overall_status        = if ($allPass) { "PASS" } else { "FAIL" }
  test_classifications = $testResults
  details               = $evidenceDetails
}

if ($Json) {
  $summary | ConvertTo-Json -Depth 5
} else {
  Write-Host "=== Subphase S3.P1.SP3 Test Classification Summary ==="
  Write-Host "Overall Status: $($summary.overall_status)"
}

if (-not $allPass) {
  exit 1
}
exit 0
