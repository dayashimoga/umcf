<#
.SYNOPSIS
  Executes classified five-tier verification tests for Subphase S7.P1.SP2:
  - Unit: Convergent authenticated encryption & cross-namespace isolation,
          RS(4, 2) & RS(6, 3) combinatorial missing shard reconstruction (all 152 combinations),
          BLAKE3 pre-reconstruction corruption detection, and failure domain fitness scoring.
  - Integration: 6-node primary upload, 2 node outage, instant download, and self-healing repair to 7th node.
  - Simulation: 20-node cluster churn simulation (100 objects, 5 churn epochs, 0 data loss).
  - Hardware-Required: SIMD Reed-Solomon RS(4, 2) encode and decode throughput benchmark.
  - Manually-Verified: Shard Health Matrix UI schema and status badge verification.
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
Write-Host "[1/5] Running [Unit] tests for convergent cipher, RS(4,2)/RS(6,3) combinations, BLAKE3 guard, and placement..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $u1 = & cargo test -p umcf-storage --test erasure_placement_tests test_convergent_xchacha20poly1305_ 2>&1 | Out-String
  $pass1 = ($LASTEXITCODE -eq 0)
  $u2 = & cargo test -p umcf-storage --test erasure_placement_tests test_rs_4_2_and_6_3_all_combinations_ 2>&1 | Out-String
  $pass2 = ($LASTEXITCODE -eq 0)
  $u3 = & cargo test -p umcf-storage --test erasure_placement_tests test_corrupted_shard_detected_ 2>&1 | Out-String
  $pass3 = ($LASTEXITCODE -eq 0)
  $u4 = & cargo test -p umcf-storage --test erasure_placement_tests test_failure_domain_placement_ 2>&1 | Out-String
  $pass4 = ($LASTEXITCODE -eq 0)

  $ErrorActionPreference = $prevEap

  if ($pass1 -and $pass2 -and $pass3 -and $pass4) {
    $testResults.unit = "PASS"
    $evidenceDetails["unit"] = "Convergent cipher deduplication/isolation, RS(4,2) & RS(6,3) all 152 combinations verified, BLAKE3 pre-inversion guard, and multi-attribute placement fitness scoring PASS"
    Write-Host "  -> [Unit] PASS" -ForegroundColor Green
  } else {
    Write-Host "  -> [Unit] FAIL`n$u1`n$u2`n$u3`n$u4" -ForegroundColor Red
  }
} catch {
  Write-Host "  -> [Unit] ERROR: $_" -ForegroundColor Red
}

# 2. INTEGRATION TESTS
Write-Host "[2/5] Running [Integration] tests for 6-node RS(4,2) upload, 2 node outage, and self-heal..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $intOut = & cargo test -p umcf-storage --test erasure_placement_tests test_6_node_rs_4_2_upload_kill_2_nodes_download_and_self_heal_ 2>&1 | Out-String
  $intPass = ($LASTEXITCODE -eq 0)

  $ErrorActionPreference = $prevEap

  if ($intPass) {
    $testResults.integration = "PASS"
    $evidenceDetails["integration"] = "6-node RS(4,2) upload, killing 2 nodes preserves instantaneous download, self-healing restores 6/6 shards on 7th spare node PASS"
    Write-Host "  -> [Integration] PASS" -ForegroundColor Green
  } else {
    Write-Host "  -> [Integration] FAIL`n$intOut" -ForegroundColor Red
  }
} catch {
  Write-Host "  -> [Integration] ERROR: $_" -ForegroundColor Red
}

# 3. SIMULATION TESTS
Write-Host "[3/5] Running [Simulation] tests for 20-node cluster churn with 100 objects and 0 data loss..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $simOut = & cargo test -p umcf-storage --test erasure_placement_tests test_20_node_cluster_churn_simulation_ 2>&1 | Out-String
  $simPass = ($LASTEXITCODE -eq 0)

  $ErrorActionPreference = $prevEap

  if ($simPass) {
    $testResults.simulation = "PASS"
    $evidenceDetails["simulation"] = "20-node cluster churn simulation across 5 epochs with 100 stored objects: 0 data loss, 100% availability PASS"
    Write-Host "  -> [Simulation] PASS" -ForegroundColor Green
  } else {
    Write-Host "  -> [Simulation] FAIL`n$simOut" -ForegroundColor Red
  }
} catch {
  Write-Host "  -> [Simulation] ERROR: $_" -ForegroundColor Red
}

# 4. HARDWARE-REQUIRED TESTS
Write-Host "[4/5] Running [Hardware-Required] tests for SIMD Reed-Solomon RS(4,2) encode/decode throughput..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $hwOut = & cargo test -p umcf-storage --test erasure_placement_tests test_hardware_simd_reed_solomon_ -- --nocapture 2>&1 | Out-String
  $hwPass = ($LASTEXITCODE -eq 0)

  $ErrorActionPreference = $prevEap

  if ($hwPass) {
    $testResults.hardware_required = "PASS"
    $evidenceDetails["hardware_required"] = "SIMD Reed-Solomon RS(4,2) throughput verified on host hardware PASS"
    Write-Host "  -> [Hardware-Required] PASS" -ForegroundColor Green
  } else {
    Write-Host "  -> [Hardware-Required] FAIL`n$hwOut" -ForegroundColor Red
  }
} catch {
  Write-Host "  -> [Hardware-Required] ERROR: $_" -ForegroundColor Red
}

# 5. MANUALLY-VERIFIED TESTS
Write-Host "[5/5] Running [Manually-Verified] tests for Shard Health Matrix UI schema..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $manOut = & cargo test -p umcf-storage --test erasure_placement_tests test_manual_shard_health_matrix_ 2>&1 | Out-String
  $manPass = ($LASTEXITCODE -eq 0)

  $ErrorActionPreference = $prevEap

  if ($manPass) {
    $testResults.manually_verified = "PASS"
    $evidenceDetails["manually_verified"] = "Shard Health Matrix UI schema and status badge verification PASS"
    Write-Host "  -> [Manually-Verified] PASS" -ForegroundColor Green
  } else {
    Write-Host "  -> [Manually-Verified] FAIL`n$manOut" -ForegroundColor Red
  }
} catch {
  Write-Host "  -> [Manually-Verified] ERROR: $_" -ForegroundColor Red
}

Write-Host "`n=== UMCF Subphase S7.P1.SP2 Verification Summary ===" -ForegroundColor Cyan
$allPass = $true
foreach ($tier in $testResults.Keys) {
  $status = $testResults[$tier]
  if ($status -eq "PASS") {
    Write-Host "  $($tier.PadRight(20)): PASS" -ForegroundColor Green
  } else {
    Write-Host "  $($tier.PadRight(20)): FAIL" -ForegroundColor Red
    $allPass = $false
  }
}

if ($allPass) {
  Write-Host "`nSubphase S7.P1.SP2 Verification Status: 100% PASS" -ForegroundColor Green
} else {
  Write-Host "`nSubphase S7.P1.SP2 Verification Status: FAILED" -ForegroundColor Red
  exit 1
}
