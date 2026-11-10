<#
.SYNOPSIS
  Executes classified tests for Subphase S3.P1.SP2:
  - Unit: QuicStreamType encoding/decoding, RFC 8410 PKCS#8 DER conversion, and X.509 cert info extraction with CRL checks
  - Integration: Quinn/QUIC mTLS handshake, 2-RTT mutual attestation (< 15 ms in release), 64 concurrent RPC streams, and 16 MiB BLAKE3 bulk transfer
  - Simulation: Revoked certificate rejection and foreign cluster handshake termination
  - Hardware-Required: Loopback UDP socket probe and Podman storage quota compliance (< 29.0 GiB)
  - Manually-Verified: quic_throughput_benchmark.json audit (throughput, handshake latency, integrity)
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
Write-Host "[1/5] Running [Unit] tests for stream types, PKCS#8 DER, and cert info extraction..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $unitOut = & cargo test -p umcf-net -j 2 --lib -- test_quic_stream_type test_ed25519_pkcs8 test_x509_cert_info 2>&1 | Out-String
  $pass = ($LASTEXITCODE -eq 0)

  $ErrorActionPreference = $prevEap

  if ($pass) {
    $testResults.unit = "PASS"
    $evidenceDetails["unit"] = "QuicStreamType encoding/decoding PASS; Ed25519 PKCS#8 DER conversion PASS; X.509 SAN did:key extraction and CRL integration PASS"
    Write-Host "  -> [Unit] PASS" -ForegroundColor Green
  } else {
    Write-Host "  -> [Unit] FAIL`n$unitOut" -ForegroundColor Red
  }
} catch {
  Write-Host "  -> [Unit] ERROR: $_" -ForegroundColor Red
}

# 2. INTEGRATION TESTS
Write-Host "[2/5] Running [Integration] QUIC mTLS handshake, mutual attestation, 64 concurrent RPCs, and 16 MiB bulk transfer..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $intOut = & cargo test -p umcf-net -j 2 --release --lib tests::test_quic_mtls_handshake_and_mutual_attestation -- --nocapture 2>&1 | Out-String
  $intPass = ($LASTEXITCODE -eq 0)

  $ErrorActionPreference = $prevEap

  if ($intPass) {
    $testResults.integration = "PASS"
    $evidenceDetails["integration"] = "QUIC mTLS handshake + mutual attestation completed in < 15 ms; 64 concurrent bidirectional RPC streams PASS; 16 MiB bulk stream transfer with BLAKE3 verification PASS"
    Write-Host "  -> [Integration] PASS" -ForegroundColor Green
  } else {
    Write-Host "  -> [Integration] FAIL`n$intOut" -ForegroundColor Red
  }
} catch {
  Write-Host "  -> [Integration] ERROR: $_" -ForegroundColor Red
}

# 3. SIMULATION / FAULT-INJECTION TESTS
Write-Host "[3/5] Running [Simulation] revoked certificate rejection and foreign cluster handshake termination..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $simOut = & cargo test -p umcf-net -j 2 --lib -- test_quic_mtls_revoked_cert test_quic_mtls_foreign_cluster 2>&1 | Out-String
  $simPass = ($LASTEXITCODE -eq 0)

  $ErrorActionPreference = $prevEap

  if ($simPass) {
    $testResults.simulation = "PASS"
    $evidenceDetails["simulation"] = "Revoked node certificate rejected at TLS handshake via ClusterCrl; Foreign cluster certificate rejected at handshake"
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
Write-Host "[5/5] Running [Manually-Verified] quic_throughput_benchmark.json audit..."
try {
  $benchPath = Join-Path $RepoRoot "artifacts/evidence/stage-03/quic_throughput_benchmark.json"

  if (Test-Path $benchPath) {
    $bench = Get-Content $benchPath -Raw | ConvertFrom-Json

    $throughputConfirmed = ($bench.throughput_gbps -gt 0)
    $integrityConfirmed = ($bench.integrity_verified -eq $true)
    $handshakeConfirmed = ($bench.handshake_attestation_ms -le 15.0)
    $statusPass = ($bench.status -eq "PASS")

    if ($throughputConfirmed -and $integrityConfirmed -and $handshakeConfirmed -and $statusPass) {
      $testResults.manually_verified = "PASS"
      $evidenceDetails["manually_verified"] = "quic_throughput_benchmark.json verified: handshake_attestation_ms ($($bench.handshake_attestation_ms) ms <= 15.0 ms); integrity_verified = true; status = PASS"
      Write-Host "  -> [Manually-Verified] PASS" -ForegroundColor Green
    } else {
      Write-Host "  -> [Manually-Verified] FAIL (Benchmark metrics invalid: handshake=$($bench.handshake_attestation_ms), integrity=$($bench.integrity_verified), status=$($bench.status))" -ForegroundColor Red
    }
  } else {
    Write-Host "  -> [Manually-Verified] FAIL (quic_throughput_benchmark.json missing)" -ForegroundColor Red
  }
} catch {
  Write-Host "  -> [Manually-Verified] ERROR: $_" -ForegroundColor Red
}

$allPass = ($testResults.unit -eq "PASS") -and ($testResults.integration -eq "PASS") -and ($testResults.simulation -eq "PASS") -and ($testResults.hardware_required -eq "PASS") -and ($testResults.manually_verified -eq "PASS")

$summary = [ordered]@{
  subphase              = "S3.P1.SP2"
  overall_status        = if ($allPass) { "PASS" } else { "FAIL" }
  test_classifications = $testResults
  details               = $evidenceDetails
}

if ($Json) {
  $summary | ConvertTo-Json -Depth 5
} else {
  Write-Host "=== Subphase S3.P1.SP2 Test Classification Summary ==="
  Write-Host "Overall Status: $($summary.overall_status)"
}

if (-not $allPass) {
  exit 1
}
exit 0
