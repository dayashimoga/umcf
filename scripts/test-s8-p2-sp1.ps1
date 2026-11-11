<#
.SYNOPSIS
  Executes classified five-tier verification tests for Subphase S8.P2.SP1:
  - Unit: Length-prefix allocation bomb (2^62 B) rejection, deeply nested bracket recursion limits (<= 32),
          and minimized seed corpus regressions across all 15 fuzz targets.
  - Integration: Proptest property-based structured fuzzing (500 cases per target).
  - Simulation: Simulated network socket fuzzer with mutated datagrams and malformed packets.
  - Hardware-Required: Multi-core parallel fuzzing across host CPU cores.
  - Manually-Verified: Minimized corpus run summary schema and zero-crash verification.
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
Write-Host "[1/5] Running [Unit] tests for allocation bomb rejection, recursion depth, and 15-target corpus regression..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $u1 = & cargo test -p fuzz-suite --test fuzz_tests test_length_prefix_allocation_bomb_2_pow_62_rejected_with_zero_heap_alloc 2>&1 | Out-String
  $pass1 = ($LASTEXITCODE -eq 0)
  $u2 = & cargo test -p fuzz-suite --test fuzz_tests test_deeply_nested_gguf_array_and_json_schema_recursion_limit_enforced 2>&1 | Out-String
  $pass2 = ($LASTEXITCODE -eq 0)
  $u3 = & cargo test -p fuzz-suite --test fuzz_tests test_minimized_fuzz_corpus_regression_all_15_targets 2>&1 | Out-String
  $pass3 = ($LASTEXITCODE -eq 0)

  $ErrorActionPreference = $prevEap

  if ($pass1 -and $pass2 -and $pass3) {
    $testResults.unit = "PASS"
    $evidenceDetails["unit"] = "Allocation bomb rejection (2^62 bytes), recursion depth capping (<= 32), and 15-target seed corpus regression PASS"
    Write-Host "  -> [Unit] PASS" -ForegroundColor Green
  } else {
    Write-Host "  -> [Unit] FAIL`n$u1`n$u2`n$u3" -ForegroundColor Red
  }
} catch {
  Write-Host "  -> [Unit] ERROR: $_" -ForegroundColor Red
}

# 2. INTEGRATION TESTS
Write-Host "[2/5] Running [Integration] tests for proptest structured fuzzing..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $i1 = & cargo test -p fuzz-suite --test fuzz_tests test_proptest_fuzz_wire_and_tokens 2>&1 | Out-String
  $passI1 = ($LASTEXITCODE -eq 0)

  $ErrorActionPreference = $prevEap

  if ($passI1) {
    $testResults.integration = "PASS"
    $evidenceDetails["integration"] = "Property-based randomized structured fuzzing across wire frames, tokens, GGUF, and receipts PASS"
    Write-Host "  -> [Integration] PASS" -ForegroundColor Green
  } else {
    Write-Host "  -> [Integration] FAIL`n$i1" -ForegroundColor Red
  }
} catch {
  Write-Host "  -> [Integration] ERROR: $_" -ForegroundColor Red
}

# 3. SIMULATION TESTS
Write-Host "[3/5] Running [Simulation] tests for live socket simulated datagram fuzzing..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $s1 = & cargo test -p fuzz-suite --test fuzz_tests test_live_socket_simulated_fuzzer_malformed_quic_and_http 2>&1 | Out-String
  $passS1 = ($LASTEXITCODE -eq 0)

  $ErrorActionPreference = $prevEap

  if ($passS1) {
    $testResults.simulation = "PASS"
    $evidenceDetails["simulation"] = "200 mutated datagrams blasted at wire and stream parsers with 100% clean rejection PASS"
    Write-Host "  -> [Simulation] PASS" -ForegroundColor Green
  } else {
    Write-Host "  -> [Simulation] FAIL`n$s1" -ForegroundColor Red
  }
} catch {
  Write-Host "  -> [Simulation] ERROR: $_" -ForegroundColor Red
}

# 4. HARDWARE-REQUIRED TESTS
Write-Host "[4/5] Running [Hardware-Required] tests for multi-core parallel fuzzing execution..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $h1 = & cargo test -p fuzz-suite --test fuzz_tests test_hardware_multicore_parallel_fuzz_stability 2>&1 | Out-String
  $passH1 = ($LASTEXITCODE -eq 0)

  $ErrorActionPreference = $prevEap

  if ($passH1) {
    $testResults.hardware_required = "PASS"
    $evidenceDetails["hardware_required"] = "Multi-threaded parallel fuzz execution across physical CPU cores PASS"
    Write-Host "  -> [Hardware-Required] PASS" -ForegroundColor Green
  } else {
    Write-Host "  -> [Hardware-Required] FAIL`n$h1" -ForegroundColor Red
  }
} catch {
  Write-Host "  -> [Hardware-Required] ERROR: $_" -ForegroundColor Red
}

# 5. MANUALLY-VERIFIED TESTS
Write-Host "[5/5] Running [Manually-Verified] tests for fuzz summary and coverage schema..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $m1 = & cargo test -p fuzz-suite --test fuzz_tests test_manual_fuzz_summary_report_and_coverage_schema 2>&1 | Out-String
  $passM1 = ($LASTEXITCODE -eq 0)

  $ErrorActionPreference = $prevEap

  if ($passM1) {
    $testResults.manually_verified = "PASS"
    $evidenceDetails["manually_verified"] = "Corpus summary schema and zero crash/panic verification PASS"
    Write-Host "  -> [Manually-Verified] PASS" -ForegroundColor Green
  } else {
    Write-Host "  -> [Manually-Verified] FAIL`n$m1" -ForegroundColor Red
  }
} catch {
  Write-Host "  -> [Manually-Verified] ERROR: $_" -ForegroundColor Red
}

# SUMMARY
Write-Host "`n=== S8.P2.SP1 Protocol Parser & FFI Fuzzing Five-Tier Verification Summary ===" -ForegroundColor Cyan
$allPassed = $true
foreach ($tier in $testResults.Keys) {
  $status = $testResults[$tier]
  $color = if ($status -eq "PASS") { "Green" } else { "Red"; $allPassed = $false }
  Write-Host "  $tier : $status" -ForegroundColor $color
}

if ($Json) {
  $out = [ordered]@{
    subphase = "S8.P2.SP1"
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

