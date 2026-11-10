<#
.SYNOPSIS
  Executes classified tests for Subphase S4.P2.SP1:
  - Unit: JobDag topological sort & cycle detection, 15 synthetic node constraint pruning matrix, and RooflineCostModel LAN vs WAN tradeoffs
  - Integration: 100-task DAG scheduled across 32 heterogeneous nodes in strictly < 10.0 ms with 100% constraint satisfaction
  - Simulation: 1,000 randomized DAGs and cluster states with zero panics and 100% hard constraint adherence
  - Hardware-Required: Monotonic system clock and Podman storage quota compliance (< 29.0 GiB)
  - Manually-Verified: scheduler_placement_explain.json audit
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
Write-Host "[1/5] Running [Unit] tests for JobDag, constraint pruning matrix, and Roofline cost model..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $u1 = & cargo test -p umcf-scheduler --test scheduler_tests test_s4_p2_sp1_unit_dag_topological_sort_and_cycle_detection 2>&1 | Out-String
  $p1 = ($LASTEXITCODE -eq 0)
  $u2 = & cargo test -p umcf-scheduler --test scheduler_tests test_s4_p2_sp1_unit_constraint_solver_pruning_matrix 2>&1 | Out-String
  $p2 = ($LASTEXITCODE -eq 0)
  $u3 = & cargo test -p umcf-scheduler --test scheduler_tests test_s4_p2_sp1_unit_roofline_cost_model_lan_vs_wan_tradeoff 2>&1 | Out-String
  $p3 = ($LASTEXITCODE -eq 0)
  $pass = ($p1 -and $p2 -and $p3)

  $ErrorActionPreference = $prevEap

  if ($pass) {
    $testResults.unit = "PASS"
    $evidenceDetails["unit"] = "JobDag topological sort & cycle detection, 15 synthetic node constraint pruning matrix (privacy, thermal, backend, RAM/VRAM, SIMD, role), and RooflineCostModel LAN vs WAN tradeoffs PASS"
    Write-Host "  -> [Unit] PASS" -ForegroundColor Green
  } else {
    Write-Host "  -> [Unit] FAIL`n$u1`n$u2`n$u3" -ForegroundColor Red
  }
} catch {
  Write-Host "  -> [Unit] ERROR: $_" -ForegroundColor Red
}

# 2. INTEGRATION TESTS
Write-Host "[2/5] Running [Integration] 100-task DAG scheduled across 32 nodes strictly < 10 ms..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $i1 = & cargo test -p umcf-scheduler --test scheduler_tests test_s4_p2_sp1_integration_100_task_dag_across_32_nodes_under_10ms -- --nocapture 2>&1 | Out-String
  $intPass = ($LASTEXITCODE -eq 0)

  $ErrorActionPreference = $prevEap

  if ($intPass) {
    $testResults.integration = "PASS"
    $evidenceDetails["integration"] = "100-task DAG across 32 heterogeneous nodes scheduled in < 10.0 ms with 100% constraint satisfaction PASS"
    Write-Host "  -> [Integration] PASS" -ForegroundColor Green
  } else {
    Write-Host "  -> [Integration] FAIL`n$i1" -ForegroundColor Red
  }
} catch {
  Write-Host "  -> [Integration] ERROR: $_" -ForegroundColor Red
}

# 3. SIMULATION TESTS
Write-Host "[3/5] Running [Simulation] 1,000 randomized DAGs and cluster states..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $s1 = & cargo test -p umcf-scheduler --test scheduler_tests test_s4_p2_sp1_simulation_randomized_1000_dags_proptest_style -- --nocapture 2>&1 | Out-String
  $simPass = ($LASTEXITCODE -eq 0)

  $ErrorActionPreference = $prevEap

  if ($simPass) {
    $testResults.simulation = "PASS"
    $evidenceDetails["simulation"] = "1,000 randomized DAGs and cluster states verified with 100% constraint adherence and clean unschedulable diagnostics PASS"
    Write-Host "  -> [Simulation] PASS" -ForegroundColor Green
  } else {
    Write-Host "  -> [Simulation] FAIL`n$s1" -ForegroundColor Red
  }
} catch {
  Write-Host "  -> [Simulation] ERROR: $_" -ForegroundColor Red
}

# 4. HARDWARE-REQUIRED TESTS
Write-Host "[4/5] Running [Hardware-Required] monotonic timer and Podman quota..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $hwTest = & cargo test -p umcf-scheduler --test scheduler_tests test_s4_p2_sp1_hardware_monotonic_timer_and_quota 2>&1 | Out-String
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
Write-Host "[5/5] Running [Manually-Verified] scheduler_placement_explain.json audit..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $mvTest = & cargo test -p umcf-scheduler --test scheduler_tests test_s4_p2_sp1_manually_verified_explain_json_audit 2>&1 | Out-String
  $mvPass = ($LASTEXITCODE -eq 0)

  $explainPath = Join-Path $RepoRoot "artifacts\evidence\stage-04\scheduler_placement_explain.json"
  $explainExists = Test-Path $explainPath

  $ErrorActionPreference = $prevEap

  if ($mvPass -and $explainExists) {
    $explainData = Get-Content $explainPath -Raw | ConvertFrom-Json
    if ($explainData.assignments -and $explainData.solver_duration_micros -le 10000) {
      $testResults.manually_verified = "PASS"
      $evidenceDetails["manually_verified"] = "scheduler_placement_explain.json verified: solver latency $($explainData.solver_duration_micros) µs <= 10,000 µs, 100% explainability records present PASS"
      Write-Host "  -> [Manually-Verified] PASS" -ForegroundColor Green
    } else {
      Write-Host "  -> [Manually-Verified] FAIL: invalid explain structure or solver latency > 10 ms" -ForegroundColor Red
    }
  } else {
    Write-Host "  -> [Manually-Verified] FAIL`n$mvTest" -ForegroundColor Red
  }
} catch {
  Write-Host "  -> [Manually-Verified] ERROR: $_" -ForegroundColor Red
}

Write-Host ""
Write-Host "Test Summary for Subphase S4.P2.SP1:"
$allPass = $true
foreach ($tier in $testResults.Keys) {
  $status = $testResults[$tier]
  $color = if ($status -eq "PASS") { "Green" } else { "Red" }
  Write-Host "  $tier`: $status" -ForegroundColor $color
  if ($status -ne "PASS") { $allPass = $false }
}

if ($Json) {
  $output = [ordered]@{
    subphase = "S4.P2.SP1"
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
