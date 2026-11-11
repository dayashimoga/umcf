<#
.SYNOPSIS
  Executes classified five-tier verification tests for Subphase S7.P2.SP2:
  - Unit: Clap command tree parsing (12 root subcommands), strict `--format json` output,
          UTF-8 half-block QR code renderer, and deterministic exit code mapping.
  - Integration: End-to-end CLI workflow (storage put -> pin -> job submit -> ai run -> gc),
                 shell completions generation (bash, zsh, fish, powershell),
                 and SIGINT Ctrl+C in-flight cancellation.
  - Simulation: Top TUI headless layout across variable terminal dimensions (80x24 and 160x50).
  - Hardware-Required: Hardware pre-flight doctor diagnostics probe (CPU topology, SIMD, Podman 30GB ceiling).
  - Manually-Verified: Cluster pair QR payload schema, AI chat format, and top TUI text schema.
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
Write-Host "[1/5] Running [Unit] tests for Clap parsing, JSON formatting, QR rendering, and exit codes..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $u1 = & cargo test -p umcf-cli --test cli_tests test_clap_command_tree_parsing_all_subcommands_and_flags 2>&1 | Out-String
  $pass1 = ($LASTEXITCODE -eq 0)
  $u2 = & cargo test -p umcf-cli --test cli_tests test_json_output_mode_produces_valid_parseable_json_for_all_commands 2>&1 | Out-String
  $pass2 = ($LASTEXITCODE -eq 0)
  $u3 = & cargo test -p umcf-cli --test cli_tests test_utf8_terminal_qr_code_renderer 2>&1 | Out-String
  $pass3 = ($LASTEXITCODE -eq 0)
  $u4 = & cargo test -p umcf-cli --test cli_tests test_deterministic_exit_code_mapping_for_error_variants 2>&1 | Out-String
  $pass4 = ($LASTEXITCODE -eq 0)

  $ErrorActionPreference = $prevEap

  if ($pass1 -and $pass2 -and $pass3 -and $pass4) {
    $testResults.unit = "PASS"
    $evidenceDetails["unit"] = "All 12 subcommands parsed, strict JSON validation, UTF-8 QR generation, and deterministic exit code mapping PASS"
    Write-Host "  -> [Unit] PASS" -ForegroundColor Green
  } else {
    Write-Host "  -> [Unit] FAIL`n$u1`n$u2`n$u3`n$u4" -ForegroundColor Red
  }
} catch {
  Write-Host "  -> [Unit] ERROR: $_" -ForegroundColor Red
}

# 2. INTEGRATION TESTS
Write-Host "[2/5] Running [Integration] tests for E2E CLI workflow, shell completions, and SIGINT..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $i1 = & cargo test -p umcf-cli --test cli_tests test_cli_e2e_workflow_storage_put_job_submit_ai_run_and_gc 2>&1 | Out-String
  $passI1 = ($LASTEXITCODE -eq 0)
  $i2 = & cargo test -p umcf-cli --test cli_tests test_shell_completions_generation_bash_zsh_fish_powershell 2>&1 | Out-String
  $passI2 = ($LASTEXITCODE -eq 0)
  $i3 = & cargo test -p umcf-cli --test cli_tests test_cli_sigint_ctrl_c_cancels_remote_in_flight_job 2>&1 | Out-String
  $passI3 = ($LASTEXITCODE -eq 0)

  $ErrorActionPreference = $prevEap

  if ($passI1 -and $passI2 -and $passI3) {
    $testResults.integration = "PASS"
    $evidenceDetails["integration"] = "E2E workflow (storage->pin->job->ai->gc), 4-shell completion generators, and remote in-flight SIGINT cancel PASS"
    Write-Host "  -> [Integration] PASS" -ForegroundColor Green
  } else {
    Write-Host "  -> [Integration] FAIL`n$i1`n$i2`n$i3" -ForegroundColor Red
  }
} catch {
  Write-Host "  -> [Integration] ERROR: $_" -ForegroundColor Red
}

# 3. SIMULATION TESTS
Write-Host "[3/5] Running [Simulation] tests for top TUI headless layout..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $s1 = & cargo test -p umcf-cli --test cli_tests test_top_headless_terminal_buffer_layout_80x24_and_160x50 2>&1 | Out-String
  $passS1 = ($LASTEXITCODE -eq 0)

  $ErrorActionPreference = $prevEap

  if ($passS1) {
    $testResults.simulation = "PASS"
    $evidenceDetails["simulation"] = "Terminal layout snapshot verified across 80x24 and 160x50 dimensions without text overflow PASS"
    Write-Host "  -> [Simulation] PASS" -ForegroundColor Green
  } else {
    Write-Host "  -> [Simulation] FAIL`n$s1" -ForegroundColor Red
  }
} catch {
  Write-Host "  -> [Simulation] ERROR: $_" -ForegroundColor Red
}

# 4. HARDWARE-REQUIRED TESTS
Write-Host "[4/5] Running [Hardware-Required] tests for doctor diagnostics..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $h1 = & cargo test -p umcf-cli --test cli_tests test_hardware_doctor_diagnostics_report 2>&1 | Out-String
  $passH1 = ($LASTEXITCODE -eq 0)

  $ErrorActionPreference = $prevEap

  if ($passH1) {
    $testResults.hardware_required = "PASS"
    $evidenceDetails["hardware_required"] = "Physical hardware pre-flight doctor probe (CPU cores, SIMD, Podman 30GB ceiling) PASS"
    Write-Host "  -> [Hardware-Required] PASS" -ForegroundColor Green
  } else {
    Write-Host "  -> [Hardware-Required] FAIL`n$h1" -ForegroundColor Red
  }
} catch {
  Write-Host "  -> [Hardware-Required] ERROR: $_" -ForegroundColor Red
}

# 5. MANUALLY-VERIFIED TESTS
Write-Host "[5/5] Running [Manually-Verified] tests for QR schema, top text layout, and AI chat..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $m1 = & cargo test -p umcf-cli --test cli_tests test_manual_cluster_pair_qr_ai_chat_and_top_cli_schema 2>&1 | Out-String
  $passM1 = ($LASTEXITCODE -eq 0)

  $ErrorActionPreference = $prevEap

  if ($passM1) {
    $testResults.manually_verified = "PASS"
    $evidenceDetails["manually_verified"] = "QR URI protocol schema, interactive chat structure, and top monitor layout schema PASS"
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
Write-Host " S7.P2.SP2 Classified Test Execution Summary: $totalPassed / 5 Passed"
Write-Host "========================================================"
foreach ($key in $testResults.Keys) {
  $status = $testResults[$key]
  $color = if ($status -eq "PASS") { "Green" } else { "Red" }
  Write-Host "  - [$key]: $status" -ForegroundColor $color
}

if ($Json) {
  $report = [ordered]@{
    subphase          = "S7.P2.SP2"
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
