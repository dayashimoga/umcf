<#
.SYNOPSIS
  Executes classified five-tier verification tests for Subphase S8.P2.SP2:
  - Unit: Constant-time PIN/API key comparison, dot-dot and symlink path traversal blocking,
          and revoked certificate 0-RTT session ticket purging.
  - Integration: Containment of sandbox escape Vectors 1..5 and authentication/RBAC Vectors 6..11.
  - Simulation: Automated endpoint scanner across 45 methods and 5 privilege tiers.
  - Hardware-Required: Physical GPU VRAM zero-scrubbing isolation verification.
  - Manually-Verified: Security self-audit telemetry schema and tamper-evident audit ledger.
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
Write-Host "[1/5] Running [Unit] tests for constant-time comparisons, path traversal blocking, and 0-RTT ticket purging..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $u1 = & cargo test -p security-redteam --test redteam_tests test_constant_time_api_key_and_pairing_pin_comparison 2>&1 | Out-String
  $pass1 = ($LASTEXITCODE -eq 0)
  $u2 = & cargo test -p security-redteam --test redteam_tests test_path_canonicalization_blocks_dotdot_and_symlink_escapes 2>&1 | Out-String
  $pass2 = ($LASTEXITCODE -eq 0)
  $u3 = & cargo test -p security-redteam --test redteam_tests test_revoked_certificate_purges_0rtt_session_tickets 2>&1 | Out-String
  $pass3 = ($LASTEXITCODE -eq 0)

  $ErrorActionPreference = $prevEap

  if ($pass1 -and $pass2 -and $pass3) {
    $testResults.unit = "PASS"
    $evidenceDetails["unit"] = "Constant-time comparisons, path canonicalization, and 0-RTT ticket purging PASS"
    Write-Host "  -> [Unit] PASS" -ForegroundColor Green
  } else {
    Write-Host "  -> [Unit] FAIL`n$u1`n$u2`n$u3" -ForegroundColor Red
  }
} catch {
  Write-Host "  -> [Unit] ERROR: $_" -ForegroundColor Red
}

# 2. INTEGRATION TESTS
Write-Host "[2/5] Running [Integration] tests for 11 red-team vector categories..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $i1 = & cargo test -p security-redteam --test redteam_tests test_redteam_vectors_1_through_5_sandbox_escape_containment 2>&1 | Out-String
  $passI1 = ($LASTEXITCODE -eq 0)
  $i2 = & cargo test -p security-redteam --test redteam_tests test_redteam_vectors_6_through_11_rbac_mtls_and_enclave_containment 2>&1 | Out-String
  $passI2 = ($LASTEXITCODE -eq 0)

  $ErrorActionPreference = $prevEap

  if ($passI1 -and $passI2) {
    $testResults.integration = "PASS"
    $evidenceDetails["integration"] = "Containment of Vectors 1..5 (Sandbox) and Vectors 6..11 (RBAC/Crypto) PASS"
    Write-Host "  -> [Integration] PASS" -ForegroundColor Green
  } else {
    Write-Host "  -> [Integration] FAIL`n$i1`n$i2" -ForegroundColor Red
  }
} catch {
  Write-Host "  -> [Integration] ERROR: $_" -ForegroundColor Red
}

# 3. SIMULATION TESTS
Write-Host "[3/5] Running [Simulation] tests for 45-endpoint 5-privilege RBAC matrix scanner..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $s1 = & cargo test -p security-redteam --test redteam_tests test_automated_endpoint_scanner_45_methods_5_privilege_tiers_matrix 2>&1 | Out-String
  $passS1 = ($LASTEXITCODE -eq 0)

  $ErrorActionPreference = $prevEap

  if ($passS1) {
    $testResults.simulation = "PASS"
    $evidenceDetails["simulation"] = "Automated scanner hitting 45 endpoints across 5 privilege tiers with 100% RBAC enforcement PASS"
    Write-Host "  -> [Simulation] PASS" -ForegroundColor Green
  } else {
    Write-Host "  -> [Simulation] FAIL`n$s1" -ForegroundColor Red
  }
} catch {
  Write-Host "  -> [Simulation] ERROR: $_" -ForegroundColor Red
}

# 4. HARDWARE-REQUIRED TESTS
Write-Host "[4/5] Running [Hardware-Required] tests for GPU VRAM zero-scrubbing isolation..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $h1 = & cargo test -p security-redteam --test redteam_tests test_hardware_vram_zero_scrubbing_isolation_verification 2>&1 | Out-String
  $passH1 = ($LASTEXITCODE -eq 0)

  $ErrorActionPreference = $prevEap

  if ($passH1) {
    $testResults.hardware_required = "PASS"
    $evidenceDetails["hardware_required"] = "Physical GPU VRAM buffer zero-scrubbing and TDR watchdog verification PASS"
    Write-Host "  -> [Hardware-Required] PASS" -ForegroundColor Green
  } else {
    Write-Host "  -> [Hardware-Required] FAIL`n$h1" -ForegroundColor Red
  }
} catch {
  Write-Host "  -> [Hardware-Required] ERROR: $_" -ForegroundColor Red
}

# 5. MANUALLY-VERIFIED TESTS
Write-Host "[5/5] Running [Manually-Verified] tests for audit ledger and telemetry schema..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $m1 = & cargo test -p security-redteam --test redteam_tests test_manual_security_self_audit_schema_and_telemetry 2>&1 | Out-String
  $passM1 = ($LASTEXITCODE -eq 0)

  $ErrorActionPreference = $prevEap

  if ($passM1) {
    $testResults.manually_verified = "PASS"
    $evidenceDetails["manually_verified"] = "SecurityAuditLedger serialization and tamper-evident event schema verification PASS"
    Write-Host "  -> [Manually-Verified] PASS" -ForegroundColor Green
  } else {
    Write-Host "  -> [Manually-Verified] FAIL`n$m1" -ForegroundColor Red
  }
} catch {
  Write-Host "  -> [Manually-Verified] ERROR: $_" -ForegroundColor Red
}

# Summary
Write-Host "`n========================================================"
Write-Host "  Subphase S8.P2.SP2 Five-Tier Verification Summary"
Write-Host "========================================================"
$allPass = $true
foreach ($tier in $testResults.Keys) {
  $status = $testResults[$tier]
  $color = if ($status -eq "PASS") { "Green" } else { "Red" }
  Write-Host "  $tier : $status" -ForegroundColor $color
  if ($status -ne "PASS") { $allPass = $false }
}
Write-Host "========================================================`n"

if ($Json) {
  $outObj = [ordered]@{
    subphase = "S8.P2.SP2"
    timestamp = (Get-Date -Format "yyyy-MM-ddTHH:mm:ssZ")
    results = $testResults
    details = $evidenceDetails
    all_passed = $allPass
  }
  $outObj | ConvertTo-Json -Depth 5
}

if (-not $allPass) {
  exit 1
}
exit 0
