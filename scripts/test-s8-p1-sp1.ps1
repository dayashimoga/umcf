<#
.SYNOPSIS
  Executes classified five-tier verification tests for Subphase S8.P1.SP1:
  - Unit: VirtualPathNormalizer wire path hygiene, CaseCollisionDetector cross-filesystem hazard interception,
          and PageAlignmentValidator 16 KiB mmap alignment for Android 15 and Apple Silicon.
  - Integration: Full 6-Scenario multi-OS cluster integration suite (Windows, Linux, macOS, Android, iOS, WebBrowser)
                 covering mTLS mesh discovery, WASM determinism, pipeline-parallel LLM, GPU rendering/transcoding,
                 low-latency streaming, and erasure storage + ledger dual-receipt settlement.
  - Simulation: Heterogeneous OS clock skew (-150ms to +100ms) and page-size divergence simulation.
  - Hardware-Required: Real host OS hardware capability probe and authentic telemetry receipt issuance.
  - Manually-Verified: Cluster topology UI inspection schema across all 6 OS node targets.
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
Write-Host "[1/5] Running [Unit] tests for wire path normalization, case collision detection, and 16 KiB page alignment..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $u1 = & cargo test -p e2e-cluster --test multi_os_tests test_virtual_path_normalizer_rejects_windows_backslashes_and_drive_letters_on_wire 2>&1 | Out-String
  $pass1 = ($LASTEXITCODE -eq 0)
  $u2 = & cargo test -p e2e-cluster --test multi_os_tests test_case_collision_detector_for_cross_os_storage_manifests 2>&1 | Out-String
  $pass2 = ($LASTEXITCODE -eq 0)
  $u3 = & cargo test -p e2e-cluster --test multi_os_tests test_16kb_page_size_mmap_alignment_for_android15_and_apple_silicon 2>&1 | Out-String
  $pass3 = ($LASTEXITCODE -eq 0)

  $ErrorActionPreference = $prevEap

  if ($pass1 -and $pass2 -and $pass3) {
    $testResults.unit = "PASS"
    $evidenceDetails["unit"] = "Path normalization (Windows backslash rejection), cross-filesystem case collision prevention, and 16 KiB mmap alignment PASS"
    Write-Host "  -> [Unit] PASS" -ForegroundColor Green
  } else {
    Write-Host "  -> [Unit] FAIL`n$u1`n$u2`n$u3" -ForegroundColor Red
  }
} catch {
  Write-Host "  -> [Unit] ERROR: $_" -ForegroundColor Red
}

# 2. INTEGRATION TESTS
Write-Host "[2/5] Running [Integration] tests for complete 6-scenario multi-OS cluster integration suite..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $i1 = & cargo test -p e2e-cluster --test multi_os_tests test_scenario_1_to_6_full_multi_os_concurrent_cluster_suite 2>&1 | Out-String
  $passI1 = ($LASTEXITCODE -eq 0)

  $ErrorActionPreference = $prevEap

  if ($passI1) {
    $testResults.integration = "PASS"
    $evidenceDetails["integration"] = "Full 6-scenario concurrent execution across Windows, Linux, macOS, Android, iOS, and WebBrowser nodes PASS"
    Write-Host "  -> [Integration] PASS" -ForegroundColor Green
  } else {
    Write-Host "  -> [Integration] FAIL`n$i1" -ForegroundColor Red
  }
} catch {
  Write-Host "  -> [Integration] ERROR: $_" -ForegroundColor Red
}

# 3. SIMULATION TESTS
Write-Host "[3/5] Running [Simulation] tests for heterogeneous OS clock drift and page-size divergence..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $s1 = & cargo test -p e2e-cluster --test multi_os_tests test_simulated_heterogeneous_os_clock_and_page_size_drift 2>&1 | Out-String
  $passS1 = ($LASTEXITCODE -eq 0)

  $ErrorActionPreference = $prevEap

  if ($passS1) {
    $testResults.simulation = "PASS"
    $evidenceDetails["simulation"] = "Cross-node simulated clock skew tolerance (-150ms to +100ms) and page-size variation (4K, 16K, 64K) PASS"
    Write-Host "  -> [Simulation] PASS" -ForegroundColor Green
  } else {
    Write-Host "  -> [Simulation] FAIL`n$s1" -ForegroundColor Red
  }
} catch {
  Write-Host "  -> [Simulation] ERROR: $_" -ForegroundColor Red
}

# 4. HARDWARE-REQUIRED TESTS
Write-Host "[4/5] Running [Hardware-Required] tests for authentic host OS hardware probe and telemetry..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $h1 = & cargo test -p e2e-cluster --test multi_os_tests test_hardware_host_os_interop_telemetry_receipt 2>&1 | Out-String
  $passH1 = ($LASTEXITCODE -eq 0)

  $ErrorActionPreference = $prevEap

  if ($passH1) {
    $testResults.hardware_required = "PASS"
    $evidenceDetails["hardware_required"] = "Real host OS CPU/RAM probe and authentic dual-signed telemetry receipt PASS"
    Write-Host "  -> [Hardware-Required] PASS" -ForegroundColor Green
  } else {
    Write-Host "  -> [Hardware-Required] FAIL`n$h1" -ForegroundColor Red
  }
} catch {
  Write-Host "  -> [Hardware-Required] ERROR: $_" -ForegroundColor Red
}

# 5. MANUALLY-VERIFIED TESTS
Write-Host "[5/5] Running [Manually-Verified] tests for cluster topology UI inspector schema..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $m1 = & cargo test -p e2e-cluster --test multi_os_tests test_manual_multi_os_cluster_topology_ui_inspection 2>&1 | Out-String
  $passM1 = ($LASTEXITCODE -eq 0)

  $ErrorActionPreference = $prevEap

  if ($passM1) {
    $testResults.manually_verified = "PASS"
    $evidenceDetails["manually_verified"] = "Cluster topology inspection schema verified across all 6 OS targets PASS"
    Write-Host "  -> [Manually-Verified] PASS" -ForegroundColor Green
  } else {
    Write-Host "  -> [Manually-Verified] FAIL`n$m1" -ForegroundColor Red
  }
} catch {
  Write-Host "  -> [Manually-Verified] ERROR: $_" -ForegroundColor Red
}

# SUMMARY
Write-Host "`n=== S8.P1.SP1 Multi-OS Cluster Integration Five-Tier Verification Summary ===" -ForegroundColor Cyan
$allPassed = $true
foreach ($tier in $testResults.Keys) {
  $status = $testResults[$tier]
  $color = if ($status -eq "PASS") { "Green" } else { "Red"; $allPassed = $false }
  Write-Host "  $tier : $status" -ForegroundColor $color
}

if ($Json) {
  $out = [ordered]@{
    subphase = "S8.P1.SP1"
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
