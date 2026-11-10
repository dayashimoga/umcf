<#
.SYNOPSIS
  Executes classified tests for Subphase S3.P3.SP2:
  - Unit: PCI Vendor ID classification (NVIDIA, AMD, Intel, Apple, Qualcomm, ARM, Google) and unified memory heuristics
  - Integration: AcceleratorProber::enumerate() live hardware enumeration (< 150 ms)
  - Simulation: Graceful CPU SIMD fallback when GPU/NPU accelerators are absent
  - Hardware-Required: Physical GPU/NPU adapter query and Podman storage quota compliance (< 29.0 GiB)
  - Manually-Verified: accelerator_enumeration_report.json audit
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
Write-Host "[1/5] Running [Unit] tests for PCI vendor classification and unified memory heuristics..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $unitOut1 = & cargo test -p umcf-hwprobe --test hwprobe_tests test_unit_pci_vendor_classification 2>&1 | Out-String
  $pass1 = ($LASTEXITCODE -eq 0)
  $unitOut2 = & cargo test -p umcf-hwprobe --test hwprobe_tests test_unit_unified_memory_heuristics 2>&1 | Out-String
  $pass2 = ($LASTEXITCODE -eq 0)
  $pass = ($pass1 -and $pass2)

  $ErrorActionPreference = $prevEap

  if ($pass) {
    $testResults.unit = "PASS"
    $evidenceDetails["unit"] = "PCI vendor ID classification (NVIDIA 0x10DE, AMD 0x1002, Intel 0x8086, Apple 0x106B, Qualcomm 0x5143, ARM 0x13B5, Google 0x1AE0) and unified memory heuristics PASS"
    Write-Host "  -> [Unit] PASS" -ForegroundColor Green
  } else {
    Write-Host "  -> [Unit] FAIL`n$unitOut" -ForegroundColor Red
  }
} catch {
  Write-Host "  -> [Unit] ERROR: $_" -ForegroundColor Red
}

# 2. INTEGRATION TESTS
Write-Host "[2/5] Running [Integration] AcceleratorProber::enumerate() live query (< 150 ms)..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $intOut = & cargo test -p umcf-hwprobe --release --test hwprobe_tests test_integration_accelerator_enumeration_live -- --nocapture 2>&1 | Out-String
  $intPass = ($LASTEXITCODE -eq 0)

  $ErrorActionPreference = $prevEap

  if ($intPass) {
    $testResults.integration = "PASS"
    $evidenceDetails["integration"] = "Live AcceleratorProber::enumerate() completed in < 150 ms, returning non-empty accelerator list with valid compute units and VRAM budgets PASS"
    Write-Host "  -> [Integration] PASS" -ForegroundColor Green
  } else {
    Write-Host "  -> [Integration] FAIL`n$intOut" -ForegroundColor Red
  }
} catch {
  Write-Host "  -> [Integration] ERROR: $_" -ForegroundColor Red
}

# 3. SIMULATION / FAULT-INJECTION TESTS
Write-Host "[3/5] Running [Simulation] graceful CPU SIMD fallback..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $simOut = & cargo test -p umcf-hwprobe --test hwprobe_tests test_simulation_cuda_rocm_absence_and_graceful_cpu_fallback 2>&1 | Out-String
  $simPass = ($LASTEXITCODE -eq 0)

  $ErrorActionPreference = $prevEap

  if ($simPass) {
    $testResults.simulation = "PASS"
    $evidenceDetails["simulation"] = "Graceful fallback to ComputeBackend::CpuSimd with verified hardware and fp16/int8 execution flags PASS"
    Write-Host "  -> [Simulation] PASS" -ForegroundColor Green
  } else {
    Write-Host "  -> [Simulation] FAIL`n$simOut" -ForegroundColor Red
  }
} catch {
  Write-Host "  -> [Simulation] ERROR: $_" -ForegroundColor Red
}

# 4. HARDWARE-REQUIRED TESTS
Write-Host "[4/5] Running [Hardware-Required] physical GPU/NPU probe and Podman quota..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $hwOut = & cargo test -p umcf-hwprobe --test hwprobe_tests test_hw_required_accelerator_enumeration -- --nocapture 2>&1 | Out-String
  $hwPass = ($LASTEXITCODE -eq 0)

  $quotaOutput = & powershell -ExecutionPolicy Bypass -File (Join-Path $ScriptDir "podman-quota-check.ps1") 2>&1 | Out-String
  $quotaPass = ($LASTEXITCODE -eq 0)

  $ErrorActionPreference = $prevEap

  if ($hwPass -and $quotaPass) {
    $testResults.hardware_required = "PASS"
    $evidenceDetails["hardware_required"] = "Physical graphics adapter introspection verified; Podman storage quota compliance PASS (< 29.0 GiB)"
    Write-Host "  -> [Hardware-Required] PASS" -ForegroundColor Green
  } else {
    Write-Host "  -> [Hardware-Required] FAIL (HW: $hwPass, Quota: $quotaPass)" -ForegroundColor Red
  }
} catch {
  Write-Host "  -> [Hardware-Required] ERROR: $_" -ForegroundColor Red
}

# 5. MANUALLY-VERIFIED TESTS
Write-Host "[5/5] Running [Manually-Verified] accelerator_enumeration_report.json audit..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $auditOut = & cargo test -p umcf-hwprobe --test hwprobe_tests test_manually_verified_accelerator_enumeration_report_audit 2>&1 | Out-String
  $auditPass = ($LASTEXITCODE -eq 0)

  $ErrorActionPreference = $prevEap

  $reportPath = Join-Path $RepoRoot "artifacts/evidence/stage-03/accelerator_enumeration_report.json"
  if ($auditPass -and (Test-Path $reportPath)) {
    $report = Get-Content $reportPath -Raw | ConvertFrom-Json

    $count = $report.Count
    $first = $report[0]
    $nameValid = ($first.name.Length -gt 0)
    $vramValid = ($first.vram_bytes -gt 0)
    $unitsValid = ($first.compute_units -ge 1)

    if ($count -ge 1 -and $nameValid -and $vramValid -and $unitsValid) {
      $testResults.manually_verified = "PASS"
      $evidenceDetails["manually_verified"] = "accelerator_enumeration_report.json verified: $count accelerators discovered, primary=$($first.name) ($($first.backend), $($first.kind)), VRAM=$([math]::Round($first.vram_bytes/1MB)) MB, FP16=$($first.supports_fp16) PASS"
      Write-Host "  -> [Manually-Verified] PASS" -ForegroundColor Green
    } else {
      Write-Host "  -> [Manually-Verified] FAIL (Report invalid or empty accelerators)" -ForegroundColor Red
    }
  } else {
    Write-Host "  -> [Manually-Verified] FAIL (Audit test failed or accelerator_enumeration_report.json missing)`n$auditOut" -ForegroundColor Red
  }
} catch {
  Write-Host "  -> [Manually-Verified] ERROR: $_" -ForegroundColor Red
}

Write-Host "`nTest Summary for Subphase S3.P3.SP2:"
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
    subphase = "S3.P3.SP2"
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
