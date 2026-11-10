<#
.SYNOPSIS
  Executes classified tests for Subphase S3.P2.SP1:
  - Unit: RFC 8489 STUN binary framing, test vectors, ICE priority calculation, probe packet HMAC-BLAKE3 verification, and 6x6 NAT punchability matrix
  - Integration: PortRestrictedCone STUN gathering (< 300 ms), synchronized UDP hole punching (< 800 ms), and Quinn QUIC mTLS upgrade
  - Simulation: Symmetric NAT <-> Symmetric NAT clean detection and RequiresRelay fallback (< 1200 ms)
  - Hardware-Required: Network interface probe and Podman storage quota compliance (< 29.0 GiB)
  - Manually-Verified: nat_hole_punch_matrix.json audit and ICE candidate priority ordering verification
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
Write-Host "[1/5] Running [Unit] tests for RFC 8489 STUN, ICE candidate priority, and HMAC probe framing..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $unitOut = & cargo test -p umcf-net -j 2 --lib -- test_rfc8489_stun test_ice_candidate_priority test_hole_punch_probe test_nat_type_punchability 2>&1 | Out-String
  $pass = ($LASTEXITCODE -eq 0)

  $ErrorActionPreference = $prevEap

  if ($pass) {
    $testResults.unit = "PASS"
    $evidenceDetails["unit"] = "RFC 8489 STUN binary framing & test vectors PASS; RFC 5245 ICE priority & HMAC candidate set PASS; 75-byte probe framing PASS; 36-cell NAT matrix PASS"
    Write-Host "  -> [Unit] PASS" -ForegroundColor Green
  } else {
    Write-Host "  -> [Unit] FAIL`n$unitOut" -ForegroundColor Red
  }
} catch {
  Write-Host "  -> [Unit] ERROR: $_" -ForegroundColor Red
}

# 2. INTEGRATION TESTS
Write-Host "[2/5] Running [Integration] STUN gathering (< 300 ms), synchronized UDP hole punching (< 800 ms), and QUIC mTLS upgrade..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $intOut = & cargo test -p umcf-net -j 2 --release --lib tests::test_nat_port_restricted_cone_hole_punching_and_quic_mtls -- --nocapture 2>&1 | Out-String
  $intPass = ($LASTEXITCODE -eq 0)

  $ErrorActionPreference = $prevEap

  if ($intPass) {
    $testResults.integration = "PASS"
    $evidenceDetails["integration"] = "Embedded STUN gathering < 300 ms, PortRestrictedCone UDP hole punching < 800 ms, and Quinn QUIC mTLS connection upgrade PASS"
    Write-Host "  -> [Integration] PASS" -ForegroundColor Green
  } else {
    Write-Host "  -> [Integration] FAIL`n$intOut" -ForegroundColor Red
  }
} catch {
  Write-Host "  -> [Integration] ERROR: $_" -ForegroundColor Red
}

# 3. SIMULATION / FAULT-INJECTION TESTS
Write-Host "[3/5] Running [Simulation] Symmetric NAT clean rejection and RequiresRelay fallback..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $simOut = & cargo test -p umcf-net -j 2 --lib -- test_symmetric_nat_clean_rejection_requires_relay 2>&1 | Out-String
  $simPass = ($LASTEXITCODE -eq 0)

  $ErrorActionPreference = $prevEap

  if ($simPass) {
    $testResults.simulation = "PASS"
    $evidenceDetails["simulation"] = "Symmetric NAT <-> Symmetric NAT cleanly detected and returned RequiresRelay within fast-fail deadline (< 1200 ms)"
    Write-Host "  -> [Simulation] PASS" -ForegroundColor Green
  } else {
    Write-Host "  -> [Simulation] FAIL`n$simOut" -ForegroundColor Red
  }
} catch {
  Write-Host "  -> [Simulation] ERROR: $_" -ForegroundColor Red
}

# 4. HARDWARE-REQUIRED TESTS
Write-Host "[4/5] Running [Hardware-Required] network interface probe and Podman quota check..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $hwOut = & cargo test -p umcf-net -j 2 --lib tests::test_hardware_network_interfaces_probe -- --nocapture 2>&1 | Out-String
  $hwPass = ($LASTEXITCODE -eq 0)

  $quotaOutput = & powershell -ExecutionPolicy Bypass -File (Join-Path $ScriptDir "podman-quota-check.ps1") 2>&1 | Out-String
  $quotaPass = ($LASTEXITCODE -eq 0)

  $ErrorActionPreference = $prevEap

  if ($hwPass -and $quotaPass) {
    $testResults.hardware_required = "PASS"
    $evidenceDetails["hardware_required"] = "Physical loopback and OS UDP socket verified; Podman storage quota compliance PASS (< 29.0 GiB)"
    Write-Host "  -> [Hardware-Required] PASS" -ForegroundColor Green
  } else {
    Write-Host "  -> [Hardware-Required] FAIL (HW: $hwPass, Quota: $quotaPass)" -ForegroundColor Red
  }
} catch {
  Write-Host "  -> [Hardware-Required] ERROR: $_" -ForegroundColor Red
}

# 5. MANUALLY-VERIFIED TESTS
Write-Host "[5/5] Running [Manually-Verified] nat_hole_punch_matrix.json audit..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $auditOut = & cargo test -p umcf-net -j 2 --lib -- test_nat_hole_punch_matrix_audit 2>&1 | Out-String
  $auditPass = ($LASTEXITCODE -eq 0)

  $ErrorActionPreference = $prevEap

  $matrixPath = Join-Path $RepoRoot "artifacts/evidence/stage-03/nat_hole_punch_matrix.json"
  if ($auditPass -and (Test-Path $matrixPath)) {
    $matrix = Get-Content $matrixPath -Raw | ConvertFrom-Json

    $gatherConfirmed = ($matrix.candidate_gathering_ms -lt 300)
    $punchConfirmed = ($matrix.hole_punch_elapsed_ms -lt 800)
    $quicConfirmed = ($matrix.quic_mtls_upgraded -eq $true)
    $statusPass = ($matrix.status -eq "PASS")

    if ($gatherConfirmed -and $punchConfirmed -and $quicConfirmed -and $statusPass) {
      $testResults.manually_verified = "PASS"
      $evidenceDetails["manually_verified"] = "nat_hole_punch_matrix.json verified: gathering ($($matrix.candidate_gathering_ms) ms < 300 ms), hole punch ($($matrix.hole_punch_elapsed_ms) ms < 800 ms), QUIC upgraded = true, status = PASS"
      Write-Host "  -> [Manually-Verified] PASS" -ForegroundColor Green
    } else {
      Write-Host "  -> [Manually-Verified] FAIL (Matrix invalid: gather=$($matrix.candidate_gathering_ms), punch=$($matrix.hole_punch_elapsed_ms))" -ForegroundColor Red
    }
  } else {
    Write-Host "  -> [Manually-Verified] FAIL (Audit test failed or nat_hole_punch_matrix.json missing)`n$auditOut" -ForegroundColor Red
  }
} catch {
  Write-Host "  -> [Manually-Verified] ERROR: $_" -ForegroundColor Red
}

Write-Host "`nTest Summary for Subphase S3.P2.SP1:"
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
    subphase = "S3.P2.SP1"
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
