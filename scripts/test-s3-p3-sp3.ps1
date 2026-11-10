<#
.SYNOPSIS
  Executes classified tests for Subphase S3.P3.SP3:
  - Unit: FP32 and INT8 GEMM correctness against golden reference; NetworkMicroBench statistics computation; SignedHardwareAttestation signature verification
  - Integration: Full micro-benchmark suite execution (< 500 ms); network probe train over async duplex stream; Attestation to ClusterRegistryState CRDT sync
  - Simulation: Forged/tampered attestation rejection (tampered scores, spoofed DID, corrupted signature, malformed public key)
  - Hardware-Required: Real CPU GEMM and STREAM Triad execution on host hardware; Podman storage quota compliance (< 29.0 GiB)
  - Manually-Verified: signed_hardware_attestation_sample.json audit
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
Write-Host "[1/5] Running [Unit] tests for GEMM reference correctness, network stats, and attestation verification..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $u1 = & cargo test -p umcf-hwprobe --test hwprobe_tests test_unit_fp32_gemm_correctness_against_golden_reference 2>&1 | Out-String
  $p1 = ($LASTEXITCODE -eq 0)
  $u2 = & cargo test -p umcf-hwprobe --test hwprobe_tests test_unit_int8_gemm_correctness_against_golden_reference 2>&1 | Out-String
  $p2 = ($LASTEXITCODE -eq 0)
  $u3 = & cargo test -p umcf-hwprobe --test hwprobe_tests test_unit_network_micro_bench_statistics_computation 2>&1 | Out-String
  $p3 = ($LASTEXITCODE -eq 0)
  $u4 = & cargo test -p umcf-hwprobe --test hwprobe_tests test_unit_signed_hardware_attestation_signature_verification 2>&1 | Out-String
  $p4 = ($LASTEXITCODE -eq 0)
  $pass = ($p1 -and $p2 -and $p3 -and $p4)

  $ErrorActionPreference = $prevEap

  if ($pass) {
    $testResults.unit = "PASS"
    $evidenceDetails["unit"] = "FP32/INT8 GEMM reference correctness, NetworkMicroBench statistics calculation (min/avg/p95/jitter/burst), and SignedHardwareAttestation signature verification PASS"
    Write-Host "  -> [Unit] PASS" -ForegroundColor Green
  } else {
    Write-Host "  -> [Unit] FAIL`n$u1`n$u2`n$u3`n$u4" -ForegroundColor Red
  }
} catch {
  Write-Host "  -> [Unit] ERROR: $_" -ForegroundColor Red
}

# 2. INTEGRATION TESTS
Write-Host "[2/5] Running [Integration] full micro-benchmark suite, network probe train, and registry CRDT sync..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $i1 = & cargo test -p umcf-hwprobe --release --test hwprobe_tests test_integration_full_micro_benchmark_suite_execution -- --nocapture 2>&1 | Out-String
  $ip1 = ($LASTEXITCODE -eq 0)
  $i2 = & cargo test -p umcf-hwprobe --test hwprobe_tests test_integration_network_probe_train_async_duplex -- --nocapture 2>&1 | Out-String
  $ip2 = ($LASTEXITCODE -eq 0)
  $i3 = & cargo test -p umcf-hwprobe --test hwprobe_tests test_integration_attestation_to_cluster_registry_crdt_sync -- --nocapture 2>&1 | Out-String
  $ip3 = ($LASTEXITCODE -eq 0)
  $intPass = ($ip1 -and $ip2 -and $ip3)

  $ErrorActionPreference = $prevEap

  if ($intPass) {
    $testResults.integration = "PASS"
    $evidenceDetails["integration"] = "Full micro-benchmark suite executed in < 500 ms in release mode; 256 KiB network probe train completed over async duplex stream; Attestation attached to ClusterRegistryState CRDT sync PASS"
    Write-Host "  -> [Integration] PASS" -ForegroundColor Green
  } else {
    Write-Host "  -> [Integration] FAIL`n$i1`n$i2`n$i3" -ForegroundColor Red
  }
} catch {
  Write-Host "  -> [Integration] ERROR: $_" -ForegroundColor Red
}

# 3. SIMULATION / FAULT-INJECTION TESTS
Write-Host "[3/5] Running [Simulation] forged/tampered attestation rejection..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $simOut = & cargo test -p umcf-hwprobe --test hwprobe_tests test_simulation_forged_or_tampered_attestation_rejection 2>&1 | Out-String
  $simPass = ($LASTEXITCODE -eq 0)

  $ErrorActionPreference = $prevEap

  if ($simPass) {
    $testResults.simulation = "PASS"
    $evidenceDetails["simulation"] = "Tampered benchmark scores, spoofed DID node_id, zeroed signature, and invalid public key all rejected deterministically PASS"
    Write-Host "  -> [Simulation] PASS" -ForegroundColor Green
  } else {
    Write-Host "  -> [Simulation] FAIL`n$simOut" -ForegroundColor Red
  }
} catch {
  Write-Host "  -> [Simulation] ERROR: $_" -ForegroundColor Red
}

# 4. HARDWARE-REQUIRED TESTS
Write-Host "[4/5] Running [Hardware-Required] host CPU GEMM/STREAM execution and Podman quota..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $hwOut = & cargo test -p umcf-hwprobe --test hwprobe_tests test_hardware_required_real_cpu_gemm_and_stream_kernel -- --nocapture 2>&1 | Out-String
  $hwPass = ($LASTEXITCODE -eq 0)

  $quotaOutput = & powershell -ExecutionPolicy Bypass -File (Join-Path $ScriptDir "podman-quota-check.ps1") 2>&1 | Out-String
  $quotaPass = ($LASTEXITCODE -eq 0)

  $ErrorActionPreference = $prevEap

  if ($hwPass -and $quotaPass) {
    $testResults.hardware_required = "PASS"
    $evidenceDetails["hardware_required"] = "Physical CPU GEMM and STREAM Triad memory kernels verified on host; Podman storage quota compliance PASS (< 29.0 GiB)"
    Write-Host "  -> [Hardware-Required] PASS" -ForegroundColor Green
  } else {
    Write-Host "  -> [Hardware-Required] FAIL (HW: $hwPass, Quota: $quotaPass)" -ForegroundColor Red
  }
} catch {
  Write-Host "  -> [Hardware-Required] ERROR: $_" -ForegroundColor Red
}

# 5. MANUALLY-VERIFIED TESTS
Write-Host "[5/5] Running [Manually-Verified] signed_hardware_attestation_sample.json audit..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $auditOut = & cargo test -p umcf-hwprobe --test hwprobe_tests test_manually_verified_signed_attestation_sample_export 2>&1 | Out-String
  $auditPass = ($LASTEXITCODE -eq 0)

  $ErrorActionPreference = $prevEap

  $samplePath = Join-Path $RepoRoot "artifacts/evidence/stage-03/signed_hardware_attestation_sample.json"
  if ($auditPass -and (Test-Path $samplePath)) {
    $att = Get-Content $samplePath -Raw | ConvertFrom-Json

    $didValid = ($att.node_id -like "did:umcf:*")
    $sigValid = ($att.signature_hex.Length -eq 128)
    $pkValid = ($att.public_key_hex.Length -eq 64)
    $gflops = $att.benchmark_scores.fp32_gflops
    $memBw = $att.benchmark_scores.memory_bandwidth_gbps
    $duration = $att.benchmark_scores.execution_duration_ms

    if ($didValid -and $sigValid -and $pkValid -and ($gflops -gt 0) -and ($memBw -gt 0) -and ($duration -lt 500)) {
      $testResults.manually_verified = "PASS"
      $evidenceDetails["manually_verified"] = "signed_hardware_attestation_sample.json verified: node_id=$($att.node_id), signature=128 hex chars, public_key=64 hex chars, fp32_gflops=$gflops, memory_bw=$memBw GB/s, duration=$duration ms (< 500 ms) PASS"
      Write-Host "  -> [Manually-Verified] PASS" -ForegroundColor Green
    } else {
      Write-Host "  -> [Manually-Verified] FAIL (Attestation sample content invalid or duration >= 500 ms)" -ForegroundColor Red
    }
  } else {
    Write-Host "  -> [Manually-Verified] FAIL (Audit test failed or sample file missing)`n$auditOut" -ForegroundColor Red
  }
} catch {
  Write-Host "  -> [Manually-Verified] ERROR: $_" -ForegroundColor Red
}

Write-Host "`nTest Summary for Subphase S3.P3.SP3:"
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
    subphase = "S3.P3.SP3"
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

