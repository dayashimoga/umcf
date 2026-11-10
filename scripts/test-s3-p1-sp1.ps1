<#
.SYNOPSIS
  Executes classified tests for Subphase S3.P1.SP1:
  - Unit: SignedDiscoveryBeacon serialization, TXT records parsing, Ed25519 signature verification, TokenBucketLimiter, and ReplayFilter
  - Integration: 3-node LanDiscoveryEngine loopback discovery convergence (< 500 ms)
  - Simulation: Foreign and malformed beacon drop latency (< 50 µs) and 10,000 packet flood mitigation under bounded memory (< 4 MiB)
  - Hardware-Required: Host network interface loopback/multicast probe and Podman storage quota compliance (< 29.0 GiB)
  - Manually-Verified: lan_discovery_convergence.json audit and TXT key-value record inspection
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

# 1. UNIT TESTS: SignedDiscoveryBeacon, TokenBucketLimiter, ReplayFilter
Write-Host "[1/5] Running [Unit] tests for beacon signing, TXT records, rate limiter, and replay filter..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $unitOut = & cargo test -p umcf-net -j 2 --lib -- test_signed test_token test_replay test_tamper test_invalid 2>&1 | Out-String
  $pass = ($LASTEXITCODE -eq 0)

  $ErrorActionPreference = $prevEap

  if ($pass) {
    $testResults.unit = "PASS"
    $evidenceDetails["unit"] = "SignedDiscoveryBeacon roundtrip & Ed25519 strict verification PASS; DNS-SD TXT formatting PASS; TokenBucketLimiter (10 beacons/sec) PASS; ReplayFilter (64-packet window) PASS"
    Write-Host "  -> [Unit] PASS" -ForegroundColor Green
  } else {
    Write-Host "  -> [Unit] FAIL`n$unitOut" -ForegroundColor Red
  }
} catch {
  Write-Host "  -> [Unit] ERROR: $_" -ForegroundColor Red
}

# 2. INTEGRATION TESTS: 3-node LanDiscoveryEngine discovery convergence (< 500 ms)
Write-Host "[2/5] Running [Integration] 3-node loopback discovery convergence..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $intOut = & cargo test -p umcf-net -j 2 --lib tests::test_three_node_loopback_discovery_convergence -- --nocapture 2>&1 | Out-String
  $intPass = ($LASTEXITCODE -eq 0)

  $ErrorActionPreference = $prevEap

  if ($intPass) {
    $testResults.integration = "PASS"
    $evidenceDetails["integration"] = "Three LanDiscoveryEngine nodes mutually converged in < 500 ms on loopback with 6/6 peer pairs verified"
    Write-Host "  -> [Integration] PASS" -ForegroundColor Green
  } else {
    Write-Host "  -> [Integration] FAIL`n$intOut" -ForegroundColor Red
  }
} catch {
  Write-Host "  -> [Integration] ERROR: $_" -ForegroundColor Red
}

# 3. SIMULATION / FAULT-INJECTION TESTS: Drop latency and flood mitigation
Write-Host "[3/5] Running [Simulation] drop latency (< 50 us) and 10,000 packet flood mitigation..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $simOut = & cargo test -p umcf-net -j 2 --lib -- test_foreign_and_malformed test_beacon_flood 2>&1 | Out-String
  $simPass = ($LASTEXITCODE -eq 0)

  $ErrorActionPreference = $prevEap

  if ($simPass) {
    $testResults.simulation = "PASS"
    $evidenceDetails["simulation"] = "Foreign cluster and corrupted beacons dropped in < 50 us; 10,000 packet burst at > 10,000 pkt/sec throttled with bounded memory (< 4 MiB)"
    Write-Host "  -> [Simulation] PASS" -ForegroundColor Green
  } else {
    Write-Host "  -> [Simulation] FAIL`n$simOut" -ForegroundColor Red
  }
} catch {
  Write-Host "  -> [Simulation] ERROR: $_" -ForegroundColor Red
}

# 4. HARDWARE-REQUIRED TESTS: Network interface probe & Podman storage quota compliance
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

# 5. MANUALLY-VERIFIED TESTS: lan_discovery_convergence.json audit
Write-Host "[5/5] Running [Manually-Verified] discovery convergence manifest audit..."
try {
  $convPath = Join-Path $RepoRoot "artifacts/evidence/stage-03/lan_discovery_convergence.json"

  if (Test-Path $convPath) {
    $conv = Get-Content $convPath -Raw | ConvertFrom-Json

    $durationConfirmed = ($conv.convergence_duration_ms -lt 500)
    $pairsConfirmed = ($conv.pairs_discovered -eq 6)
    $statusPass = ($conv.status -eq "CONVERGED_SUCCESSFULLY")

    if ($durationConfirmed -and $pairsConfirmed -and $statusPass) {
      $testResults.manually_verified = "PASS"
      $evidenceDetails["manually_verified"] = "lan_discovery_convergence.json verified: duration ($($conv.convergence_duration_ms) ms) < 500 ms; pairs ($($conv.pairs_discovered)/6); status = CONVERGED_SUCCESSFULLY"
      Write-Host "  -> [Manually-Verified] PASS" -ForegroundColor Green
    } else {
      Write-Host "  -> [Manually-Verified] FAIL (Manifest metrics invalid)" -ForegroundColor Red
    }
  } else {
    Write-Host "  -> [Manually-Verified] FAIL (lan_discovery_convergence.json missing)" -ForegroundColor Red
  }
} catch {
  Write-Host "  -> [Manually-Verified] ERROR: $_" -ForegroundColor Red
}

$allPass = ($testResults.unit -eq "PASS") -and ($testResults.integration -eq "PASS") -and ($testResults.simulation -eq "PASS") -and ($testResults.hardware_required -eq "PASS") -and ($testResults.manually_verified -eq "PASS")

$summary = [ordered]@{
  subphase              = "S3.P1.SP1"
  overall_status        = if ($allPass) { "PASS" } else { "FAIL" }
  test_classifications = $testResults
  details               = $evidenceDetails
}

if ($Json) {
  $summary | ConvertTo-Json -Depth 5
} else {
  Write-Host "=== Subphase S3.P1.SP1 Test Classification Summary ==="
  Write-Host "Overall Status: $($summary.overall_status)"
}

if (-not $allPass) {
  exit 1
}
exit 0
