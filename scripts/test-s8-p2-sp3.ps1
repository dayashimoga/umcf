<#
.SYNOPSIS
  Executes classified five-tier verification tests for Subphase S8.P2.SP3:
  - Unit: SLSA v1.0 provenance and CycloneDX SBOM Ed25519 signature verification,
          tampered binary/SBOM digest rejection, and SPDX permissive license parser.
  - Integration: cargo-deny and polyglot dependency license compliance, dual-directory
                 bit-exact reproducible build comparison, and binary path leak inspection.
  - Simulation: Cluster admission rejection of unverified/modified peer binaries.
  - Hardware-Required: Full release build, CycloneDX 1.5 SBOM generation, and Release Authority signing.
  - Manually-Verified: umcf version --verify-provenance CLI and SBOM viewer schema.
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
Write-Host "[1/5] Running [Unit] tests for SLSA v1.0 provenance, tampering rejection, and SPDX license expressions..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $u1 = & cargo test -p supply-chain --test supply_chain_tests test_slsa_provenance_and_sbom_ed25519_signature_verification 2>&1 | Out-String
  $pass1 = ($LASTEXITCODE -eq 0)
  $u2 = & cargo test -p supply-chain --test supply_chain_tests test_tampered_binary_or_sbom_hash_rejected_by_provenance_verifier 2>&1 | Out-String
  $pass2 = ($LASTEXITCODE -eq 0)
  $u3 = & cargo test -p supply-chain --test supply_chain_tests test_spdx_license_expression_parser_rejects_gpl_and_agpl 2>&1 | Out-String
  $pass3 = ($LASTEXITCODE -eq 0)

  $ErrorActionPreference = $prevEap

  if ($pass1 -and $pass2 -and $pass3) {
    $testResults.unit = "PASS"
    $evidenceDetails["unit"] = "SLSA v1.0 provenance Ed25519 signature verification, digest tampering rejection, and SPDX license parser PASS"
    Write-Host "  -> [Unit] PASS" -ForegroundColor Green
  } else {
    Write-Host "  -> [Unit] FAIL`n$u1`n$u2`n$u3" -ForegroundColor Red
  }
} catch {
  Write-Host "  -> [Unit] ERROR: $_" -ForegroundColor Red
}

# 2. INTEGRATION TESTS
Write-Host "[2/5] Running [Integration] tests for license compliance, bit-exact reproducible builds, and path leak checks..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $i1 = & cargo test -p supply-chain --test supply_chain_tests test_cargo_deny_and_polyglot_dependency_license_compliance_gate 2>&1 | Out-String
  $passI1 = ($LASTEXITCODE -eq 0)
  $i2 = & cargo test -p supply-chain --test supply_chain_tests test_dual_directory_reproducible_build_bit_exact_binary_comparison 2>&1 | Out-String
  $passI2 = ($LASTEXITCODE -eq 0)
  $i3 = & cargo test -p supply-chain --test supply_chain_tests test_binary_contains_zero_host_workspace_path_strings 2>&1 | Out-String
  $passI3 = ($LASTEXITCODE -eq 0)

  $ErrorActionPreference = $prevEap

  if ($passI1 -and $passI2 -and $passI3) {
    $testResults.integration = "PASS"
    $evidenceDetails["integration"] = "cargo-deny license compliance, dual-directory bit-exact reproducible builds, and zero host path leaks PASS"
    Write-Host "  -> [Integration] PASS" -ForegroundColor Green
  } else {
    Write-Host "  -> [Integration] FAIL`n$i1`n$i2`n$i3" -ForegroundColor Red
  }
} catch {
  Write-Host "  -> [Integration] ERROR: $_" -ForegroundColor Red
}

# 3. SIMULATION TESTS
Write-Host "[3/5] Running [Simulation] tests for StrictVerifiedBuildOnly cluster admission..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $s1 = & cargo test -p supply-chain --test supply_chain_tests test_peer_rejected_in_strict_verified_build_only_cluster 2>&1 | Out-String
  $passS1 = ($LASTEXITCODE -eq 0)

  $ErrorActionPreference = $prevEap

  if ($passS1) {
    $testResults.simulation = "PASS"
    $evidenceDetails["simulation"] = "Rejection of unverified/modified peer binaries in StrictVerifiedBuildOnly mesh PASS"
    Write-Host "  -> [Simulation] PASS" -ForegroundColor Green
  } else {
    Write-Host "  -> [Simulation] FAIL`n$s1" -ForegroundColor Red
  }
} catch {
  Write-Host "  -> [Simulation] ERROR: $_" -ForegroundColor Red
}

# 4. HARDWARE-REQUIRED TESTS
Write-Host "[4/5] Running [Hardware-Required] tests for release SBOM generation and Ed25519 signing..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $h1 = & cargo test -p supply-chain --test supply_chain_tests test_full_release_sbom_generation_and_signature_verification 2>&1 | Out-String
  $passH1 = ($LASTEXITCODE -eq 0)

  $ErrorActionPreference = $prevEap

  if ($passH1) {
    $testResults.hardware_required = "PASS"
    $evidenceDetails["hardware_required"] = "Physical host release build, CycloneDX 1.5 SBOM generation, and Release Authority signing PASS"
    Write-Host "  -> [Hardware-Required] PASS" -ForegroundColor Green
  } else {
    Write-Host "  -> [Hardware-Required] FAIL`n$h1" -ForegroundColor Red
  }
} catch {
  Write-Host "  -> [Hardware-Required] ERROR: $_" -ForegroundColor Red
}

# 5. MANUALLY-VERIFIED TESTS
Write-Host "[5/5] Running [Manually-Verified] tests for umcf version --verify-provenance CLI and viewer schema..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $m1 = & cargo test -p supply-chain --test supply_chain_tests test_manual_sbom_and_licenses_viewer_schema 2>&1 | Out-String
  $passM1 = ($LASTEXITCODE -eq 0)

  $ErrorActionPreference = $prevEap

  if ($passM1) {
    $testResults.manually_verified = "PASS"
    $evidenceDetails["manually_verified"] = "umcf version --verify-provenance output and SBOM/license viewer schema PASS"
    Write-Host "  -> [Manually-Verified] PASS" -ForegroundColor Green
  } else {
    Write-Host "  -> [Manually-Verified] FAIL`n$m1" -ForegroundColor Red
  }
} catch {
  Write-Host "  -> [Manually-Verified] ERROR: $_" -ForegroundColor Red
}

# Summary
Write-Host "`n========================================================"
Write-Host "  Subphase S8.P2.SP3 Five-Tier Verification Summary"
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
    subphase = "S8.P2.SP3"
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

