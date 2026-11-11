<#
.SYNOPSIS
  Executes classified five-tier verification tests for Subphase S7.P2.SP3:
  - Unit: Manifest YAML/TOML schema parsing with deny_unknown_fields,
          topological DAG cycle detection and stage grouping (Kahn's algorithm),
          parameter and step output interpolation type checker,
          and lockfile BLAKE3 mismatch rejection under frozen lockfile.
  - Integration: All 6 built-in job templates E2E init/lock/execute,
                 and multi-step hybrid WASM/GPU/AI DAG pipeline execution.
  - Simulation: Mid-workflow failure and resumption with cached step output reuse.
  - Hardware-Required: Real hardware execution timings and duration accounting.
  - Manually-Verified: Built-in template gallery UI schema and auto-generated parameter form inspection.
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
Write-Host "[1/5] Running [Unit] tests for YAML/TOML schema, Kahn DAG cycles, interpolation, and lockfile..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $u1 = & cargo test -p umcf-manifest --test manifest_tests test_manifest_yaml_and_toml_parser_deny_unknown_fields 2>&1 | Out-String
  $pass1 = ($LASTEXITCODE -eq 0)
  $u2 = & cargo test -p umcf-manifest --test manifest_tests test_dag_cycle_detection_and_topological_stage_grouping 2>&1 | Out-String
  $pass2 = ($LASTEXITCODE -eq 0)
  $u3 = & cargo test -p umcf-manifest --test manifest_tests test_parameter_and_step_output_interpolation_type_checker 2>&1 | Out-String
  $pass3 = ($LASTEXITCODE -eq 0)
  $u4 = & cargo test -p umcf-manifest --test manifest_tests test_lockfile_blake3_mismatch_rejected_under_frozen_lockfile 2>&1 | Out-String
  $pass4 = ($LASTEXITCODE -eq 0)

  $ErrorActionPreference = $prevEap

  if ($pass1 -and $pass2 -and $pass3 -and $pass4) {
    $testResults.unit = "PASS"
    $evidenceDetails["unit"] = "Manifest schema (deny_unknown_fields), Kahn DAG cycle detection, variable interpolation type checking, and frozen lockfile enforcement PASS"
    Write-Host "  -> [Unit] PASS" -ForegroundColor Green
  } else {
    Write-Host "  -> [Unit] FAIL`n$u1`n$u2`n$u3`n$u4" -ForegroundColor Red
  }
} catch {
  Write-Host "  -> [Unit] ERROR: $_" -ForegroundColor Red
}

# 2. INTEGRATION TESTS
Write-Host "[2/5] Running [Integration] tests for all 6 built-in templates and hybrid pipeline..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $i1 = & cargo test -p umcf-manifest --test manifest_tests test_all_6_builtin_job_templates_init_lock_and_execute_e2e 2>&1 | Out-String
  $passI1 = ($LASTEXITCODE -eq 0)
  $i2 = & cargo test -p umcf-manifest --test manifest_tests test_multi_step_hybrid_wasm_gpu_ai_dag_pipeline_execution 2>&1 | Out-String
  $passI2 = ($LASTEXITCODE -eq 0)

  $ErrorActionPreference = $prevEap

  if ($passI1 -and $passI2) {
    $testResults.integration = "PASS"
    $evidenceDetails["integration"] = "All 6 built-in templates (WASM, RAG, Private AI, GPU Raytracing, AV1 Transcoding, Hybrid Media AI) and E2E DAG execution PASS"
    Write-Host "  -> [Integration] PASS" -ForegroundColor Green
  } else {
    Write-Host "  -> [Integration] FAIL`n$i1`n$i2" -ForegroundColor Red
  }
} catch {
  Write-Host "  -> [Integration] ERROR: $_" -ForegroundColor Red
}

# 3. SIMULATION TESTS
Write-Host "[3/5] Running [Simulation] tests for workflow failure resumption..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $s1 = & cargo test -p umcf-manifest --test manifest_tests test_workflow_resumption_reuses_cached_step_outputs_without_reexecution 2>&1 | Out-String
  $passS1 = ($LASTEXITCODE -eq 0)

  $ErrorActionPreference = $prevEap

  if ($passS1) {
    $testResults.simulation = "PASS"
    $evidenceDetails["simulation"] = "Workflow resumption reuses cached step CIDs without re-executing completed stages PASS"
    Write-Host "  -> [Simulation] PASS" -ForegroundColor Green
  } else {
    Write-Host "  -> [Simulation] FAIL`n$s1" -ForegroundColor Red
  }
} catch {
  Write-Host "  -> [Simulation] ERROR: $_" -ForegroundColor Red
}

# 4. HARDWARE-REQUIRED TESTS
Write-Host "[4/5] Running [Hardware-Required] tests for execution timings..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $h1 = & cargo test -p umcf-manifest --test manifest_tests test_hardware_execution_timings_hybrid_media_ai_pipeline 2>&1 | Out-String
  $passH1 = ($LASTEXITCODE -eq 0)

  $ErrorActionPreference = $prevEap

  if ($passH1) {
    $testResults.hardware_required = "PASS"
    $evidenceDetails["hardware_required"] = "Real hardware execution timings and CPU parallelism safety bounds PASS"
    Write-Host "  -> [Hardware-Required] PASS" -ForegroundColor Green
  } else {
    Write-Host "  -> [Hardware-Required] FAIL`n$h1" -ForegroundColor Red
  }
} catch {
  Write-Host "  -> [Hardware-Required] ERROR: $_" -ForegroundColor Red
}

# 5. MANUALLY-VERIFIED TESTS
Write-Host "[5/5] Running [Manually-Verified] tests for Template Gallery UI schema..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $m1 = & cargo test -p umcf-manifest --test manifest_tests test_manual_template_gallery_and_dag_schema_conformance 2>&1 | Out-String
  $passM1 = ($LASTEXITCODE -eq 0)

  $ErrorActionPreference = $prevEap

  if ($passM1) {
    $testResults.manually_verified = "PASS"
    $evidenceDetails["manually_verified"] = "Template Gallery UI schema, parameter forms, and DAG progression states PASS"
    Write-Host "  -> [Manually-Verified] PASS" -ForegroundColor Green
  } else {
    Write-Host "  -> [Manually-Verified] FAIL`n$m1" -ForegroundColor Red
  }
} catch {
  Write-Host "  -> [Manually-Verified] ERROR: $_" -ForegroundColor Red
}

# SUMMARY
$totalPassed = ($testResults.Values | Where-Object { $_ -eq "PASS" }).Count
$allPassed = ($totalPassed -eq 5)

Write-Host "`n========================================================"
Write-Host " S7.P2.SP3 Classified Test Execution Summary: $totalPassed / 5 Passed"
Write-Host "========================================================"
foreach ($key in $testResults.Keys) {
  $status = $testResults[$key]
  $color = if ($status -eq "PASS") { "Green" } else { "Red" }
  Write-Host "  - [$key]: $status" -ForegroundColor $color
}

if ($Json) {
  $report = [ordered]@{
    subphase          = "S7.P2.SP3"
    timestamp         = (Get-Date -Format "yyyy-MM-ddTHH:mm:ssZ")
    all_passed        = $allPassed
    passed_tiers      = $totalPassed
    total_tiers       = 5
    tier_results      = $testResults
    evidence_details  = $evidenceDetails
  }
  $report | ConvertTo-Json -Depth 5
}

if (-not $allPassed) {
  exit 1
}
exit 0

