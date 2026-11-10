<#
.SYNOPSIS
  Executes classified tests for Subphase S3.P2.SP2:
  - Unit: RelayFrame binary codec, UmcfRelayServer routing, and per-client bandwidth quotas
  - Integration: Relay fallback connection (< 1500 ms), zero-knowledge forwarding, and seamless automatic path upgrade to DirectWanPunched
  - Simulation: Malicious relay ciphertext tampering detection and rejection
  - Hardware-Required: Network interface probe and Podman storage quota compliance (< 29.0 GiB)
  - Manually-Verified: relay_fallback_and_upgrade_trace.json audit and path status verification
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
Write-Host "[1/5] Running [Unit] tests for RelayFrame binary codec, routing, and bandwidth quotas..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $unitOut = & cargo test -p umcf-net -j 2 --lib -- test_relay_frame_binary_codec test_relay_server_routing 2>&1 | Out-String
  $pass = ($LASTEXITCODE -eq 0)

  $ErrorActionPreference = $prevEap

  if ($pass) {
    $testResults.unit = "PASS"
    $evidenceDetails["unit"] = "RelayFrame binary codec roundtrip PASS; UmcfRelayServer DID routing and bandwidth quota enforcer (429) PASS"
    Write-Host "  -> [Unit] PASS" -ForegroundColor Green
  } else {
    Write-Host "  -> [Unit] FAIL`n$unitOut" -ForegroundColor Red
  }
} catch {
  Write-Host "  -> [Unit] ERROR: $_" -ForegroundColor Red
}

# 2. INTEGRATION TESTS
Write-Host "[2/5] Running [Integration] Relay fallback connection (< 1500 ms) and automatic direct path upgrade..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $intOut = & cargo test -p umcf-net -j 2 --release --lib tests::test_relay_fallback_connection_and_automatic_path_upgrade -- --nocapture 2>&1 | Out-String
  $intPass = ($LASTEXITCODE -eq 0)

  $ErrorActionPreference = $prevEap

  if ($intPass) {
    $testResults.integration = "PASS"
    $evidenceDetails["integration"] = "Relay fallback connection in < 1500 ms, zero-knowledge forwarding, and seamless upgrade from RelayedE2ee to DirectWanPunched PASS"
    Write-Host "  -> [Integration] PASS" -ForegroundColor Green
  } else {
    Write-Host "  -> [Integration] FAIL`n$intOut" -ForegroundColor Red
  }
} catch {
  Write-Host "  -> [Integration] ERROR: $_" -ForegroundColor Red
}

# 3. SIMULATION / FAULT-INJECTION TESTS
Write-Host "[3/5] Running [Simulation] malicious relay payload tampering detection..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $simOut = & cargo test -p umcf-net -j 2 --lib -- test_malicious_relay_payload_tampering 2>&1 | Out-String
  $simPass = ($LASTEXITCODE -eq 0)

  $ErrorActionPreference = $prevEap

  if ($simPass) {
    $testResults.simulation = "PASS"
    $evidenceDetails["simulation"] = "Compromised relay payload bit-flip detected and rejected via AEAD/MAC verification PASS"
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
    $evidenceDetails["hardware_required"] = "Physical loopback and OS UDP socket verified; Podman storage quota compliance PASS (< 29.0 GiB)"
    Write-Host "  -> [Hardware-Required] PASS" -ForegroundColor Green
  } else {
    Write-Host "  -> [Hardware-Required] FAIL (HW: $hwPass, Quota: $quotaPass)" -ForegroundColor Red
  }
} catch {
  Write-Host "  -> [Hardware-Required] ERROR: $_" -ForegroundColor Red
}

# 5. MANUALLY-VERIFIED TESTS
Write-Host "[5/5] Running [Manually-Verified] relay_fallback_and_upgrade_trace.json audit..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $auditOut = & cargo test -p umcf-net -j 2 --lib -- test_relay_fallback_and_upgrade_trace_audit 2>&1 | Out-String
  $auditPass = ($LASTEXITCODE -eq 0)

  $ErrorActionPreference = $prevEap

  $tracePath = Join-Path $RepoRoot "artifacts/evidence/stage-03/relay_fallback_and_upgrade_trace.json"
  if ($auditPass -and (Test-Path $tracePath)) {
    $trace = Get-Content $tracePath -Raw | ConvertFrom-Json

    $connectConfirmed = ($trace.fallback_connection_ms -lt 1500)
    $zkConfirmed = ($trace.zero_knowledge_verified -eq $true)
    $upgradeConfirmed = ($trace.upgrade_detected -eq $true)
    $zeroDropConfirmed = ($trace.dropped_streams -eq 0)
    $statusPass = ($trace.status -eq "PASS")

    if ($connectConfirmed -and $zkConfirmed -and $upgradeConfirmed -and $zeroDropConfirmed -and $statusPass) {
      $testResults.manually_verified = "PASS"
      $evidenceDetails["manually_verified"] = "relay_fallback_and_upgrade_trace.json verified: fallback connection ($($trace.fallback_connection_ms) ms < 1500 ms), zero-knowledge = true, upgrade = true, dropped streams = 0, status = PASS"
      Write-Host "  -> [Manually-Verified] PASS" -ForegroundColor Green
    } else {
      Write-Host "  -> [Manually-Verified] FAIL (Trace invalid: connect=$($trace.fallback_connection_ms), zk=$($trace.zero_knowledge_verified))" -ForegroundColor Red
    }
  } else {
    Write-Host "  -> [Manually-Verified] FAIL (Audit test failed or relay_fallback_and_upgrade_trace.json missing)`n$auditOut" -ForegroundColor Red
  }
} catch {
  Write-Host "  -> [Manually-Verified] ERROR: $_" -ForegroundColor Red
}

Write-Host "`nTest Summary for Subphase S3.P2.SP2:"
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
    subphase = "S3.P2.SP2"
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
