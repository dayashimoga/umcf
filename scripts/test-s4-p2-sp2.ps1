<#
.SYNOPSIS
  Executes classified tests for Subphase S4.P2.SP2:
  - Unit: AdaptivePartitioner water-filling proportional allocation and DataLocalityOptimizer aggregation placement
  - Integration: 20:4:1 heterogeneous cluster achieving >= 1.18x speedup over single Desktop and >= 3.2x speedup over uniform 1/3 partitioning
  - Simulation: WorkStealingCoordinator tail straggler mitigation (>= 50% skew reduction) and WAN transfer latency pruning
  - Hardware-Required: Monotonic system clock and Podman storage quota compliance (< 29.0 GiB)
  - Manually-Verified: heterogeneous_partition_speedup.json audit (Gantt chart and speedup factors)
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
Write-Host "[1/5] Running [Unit] tests for water-filling partitioner and data locality optimizer..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $u1 = & cargo test -p umcf-scheduler --test scheduler_tests test_s4_p2_sp2_unit_water_filling_partition_ratio 2>&1 | Out-String
  $pass = ($LASTEXITCODE -eq 0)

  $ErrorActionPreference = $prevEap

  if ($pass) {
    $testResults.unit = "PASS"
    $evidenceDetails["unit"] = "Water-filling proportional allocation across 20:4:1 cluster and DataLocalityOptimizer aggregation node selection PASS"
    Write-Host "  -> [Unit] PASS" -ForegroundColor Green
  } else {
    Write-Host "  -> [Unit] FAIL`n$u1" -ForegroundColor Red
  }
} catch {
  Write-Host "  -> [Unit] ERROR: $_" -ForegroundColor Red
}

# 2. INTEGRATION TESTS
Write-Host "[2/5] Running [Integration] heterogeneous speedup over single desktop and uniform 1/3..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $i1 = & cargo test -p umcf-scheduler --test scheduler_tests test_s4_p2_sp2_integration_speedup_over_desktop_and_uniform -- --nocapture 2>&1 | Out-String
  $intPass = ($LASTEXITCODE -eq 0)

  $ErrorActionPreference = $prevEap

  if ($intPass) {
    $testResults.integration = "PASS"
    $evidenceDetails["integration"] = "Heterogeneous cluster achieved >= 1.18x speedup over single Desktop and >= 3.2x speedup over uniform 1/3 partitioning with < 10% wall-clock skew PASS"
    Write-Host "  -> [Integration] PASS" -ForegroundColor Green
  } else {
    Write-Host "  -> [Integration] FAIL`n$i1" -ForegroundColor Red
  }
} catch {
  Write-Host "  -> [Integration] ERROR: $_" -ForegroundColor Red
}

# 3. SIMULATION TESTS
Write-Host "[3/5] Running [Simulation] work stealing straggler mitigation and WAN pruning..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $s1 = & cargo test -p umcf-scheduler --test scheduler_tests test_s4_p2_sp2_simulation_work_stealing_straggler_mitigation -- --nocapture 2>&1 | Out-String
  $sp1 = ($LASTEXITCODE -eq 0)
  $s2 = & cargo test -p umcf-scheduler --test scheduler_tests test_s4_p2_sp2_simulation_wan_transfer_overhead_pruning -- --nocapture 2>&1 | Out-String
  $sp2 = ($LASTEXITCODE -eq 0)
  $simPass = ($sp1 -and $sp2)

  $ErrorActionPreference = $prevEap

  if ($simPass) {
    $testResults.simulation = "PASS"
    $evidenceDetails["simulation"] = "WorkStealingCoordinator reduced tail straggler skew by >= 50% and WAN transfer latency automatically pruned slow nodes PASS"
    Write-Host "  -> [Simulation] PASS" -ForegroundColor Green
  } else {
    Write-Host "  -> [Simulation] FAIL`n$s1`n$s2" -ForegroundColor Red
  }
} catch {
  Write-Host "  -> [Simulation] ERROR: $_" -ForegroundColor Red
}

# 4. HARDWARE-REQUIRED TESTS
Write-Host "[4/5] Running [Hardware-Required] monotonic timer and Podman quota..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $hwTest = & cargo test -p umcf-scheduler --test scheduler_tests test_s4_p2_sp2_hardware_monotonic_timer_and_quota 2>&1 | Out-String
  $hwPass = ($LASTEXITCODE -eq 0)

  $quotaScript = Join-Path $RepoRoot "scripts\podman-quota-check.ps1"
  $quotaOut = & powershell -ExecutionPolicy Bypass -File $quotaScript -Json 2>&1 | Out-String
  $quotaPass = ($LASTEXITCODE -eq 0)

  $ErrorActionPreference = $prevEap

  if ($hwPass -and $quotaPass) {
    $testResults.hardware_required = "PASS"
    $evidenceDetails["hardware_required"] = "Monotonic timer and Podman storage quota compliance verified PASS"
    Write-Host "  -> [Hardware-Required] PASS" -ForegroundColor Green
  } else {
    Write-Host "  -> [Hardware-Required] FAIL`n$hwTest`n$quotaOut" -ForegroundColor Red
  }
} catch {
  Write-Host "  -> [Hardware-Required] ERROR: $_" -ForegroundColor Red
}

# 5. MANUALLY-VERIFIED TESTS
Write-Host "[5/5] Running [Manually-Verified] heterogeneous_partition_speedup.json audit..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $mvTest = & cargo test -p umcf-scheduler --test scheduler_tests test_s4_p2_sp2_manually_verified_speedup_evidence_export 2>&1 | Out-String
  $mvPass = ($LASTEXITCODE -eq 0)

  $speedupPath = Join-Path $RepoRoot "artifacts\evidence\stage-04\heterogeneous_partition_speedup.json"
  $speedupExists = Test-Path $speedupPath

  $ErrorActionPreference = $prevEap

  if ($mvPass -and $speedupExists) {
    $speedupData = Get-Content $speedupPath -Raw | ConvertFrom-Json
    if ($speedupData.speedup_over_single_fastest -ge 1.18 -and $speedupData.speedup_over_uniform -ge 3.20) {
      $testResults.manually_verified = "PASS"
      $evidenceDetails["manually_verified"] = "heterogeneous_partition_speedup.json verified: speedup over single desktop $($speedupData.speedup_over_single_fastest)x >= 1.18x, speedup over uniform $($speedupData.speedup_over_uniform)x >= 3.20x PASS"
      Write-Host "  -> [Manually-Verified] PASS" -ForegroundColor Green
    } else {
      Write-Host "  -> [Manually-Verified] FAIL: speedup targets not met in JSON evidence" -ForegroundColor Red
    }
  } else {
    Write-Host "  -> [Manually-Verified] FAIL`n$mvTest" -ForegroundColor Red
  }
} catch {
  Write-Host "  -> [Manually-Verified] ERROR: $_" -ForegroundColor Red
}

Write-Host ""
Write-Host "Test Summary for Subphase S4.P2.SP2:"
$allPass = $true
foreach ($tier in $testResults.Keys) {
  $status = $testResults[$tier]
  $color = if ($status -eq "PASS") { "Green" } else { "Red" }
  Write-Host "  $tier`: $status" -ForegroundColor $color
  if ($status -ne "PASS") { $allPass = $false }
}

if ($Json) {
  $output = [ordered]@{
    subphase = "S4.P2.SP2"
    all_pass = $allPass
    tests    = $testResults
    details  = $evidenceDetails
  }
  $output | ConvertTo-Json -Depth 4
}

if ($allPass) {
  exit 0
} else {
  exit 1
}

