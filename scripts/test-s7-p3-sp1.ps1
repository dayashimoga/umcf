<#
.SYNOPSIS
  Executes classified five-tier verification tests for Subphase S7.P3.SP1:
  - Unit: Canonical deterministic serialization, dual Ed25519 signatures,
          hash chain sequence/prev_id verification, and SanityBoundsChecker bounds enforcement.
  - Integration: E2E WASM, AI, GPU, and Media task receipts with dual countersignatures,
                 and 60s streaming incremental checkpoint micro-receipts.
  - Simulation: Malicious worker replay and inflated compute claims audit log emission.
  - Hardware-Required: Ed25519 batch signature verification throughput on host hardware.
  - Manually-Verified: Cryptographic Receipt Explorer card schema and dual-signature verified modal.
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
Write-Host "[1/5] Running [Unit] tests for canonical serialization, hash chains, and sanity bounds..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $u1 = & cargo test -p umcf-ledger --test ledger_tests test_canonical_receipt_serialization_and_dual_ed25519_signatures 2>&1 | Out-String
  $pass1 = ($LASTEXITCODE -eq 0)
  $u2 = & cargo test -p umcf-ledger --test ledger_tests test_receipt_hash_chain_detects_missing_or_reordered_sequence 2>&1 | Out-String
  $pass2 = ($LASTEXITCODE -eq 0)
  $u3 = & cargo test -p umcf-ledger --test ledger_tests test_sanity_bounds_checker_rejects_inflated_token_or_pixel_claims 2>&1 | Out-String
  $pass3 = ($LASTEXITCODE -eq 0)

  $ErrorActionPreference = $prevEap

  if ($pass1 -and $pass2 -and $pass3) {
    $testResults.unit = "PASS"
    $evidenceDetails["unit"] = "Canonical deterministic serialization, Ed25519 dual signatures, hash-chain integrity verification, and SanityBoundsChecker bounds enforcement PASS"
    Write-Host "  -> [Unit] PASS" -ForegroundColor Green
  } else {
    Write-Host "  -> [Unit] FAIL`n$u1`n$u2`n$u3" -ForegroundColor Red
  }
} catch {
  Write-Host "  -> [Unit] ERROR: $_" -ForegroundColor Red
}

# 2. INTEGRATION TESTS
Write-Host "[2/5] Running [Integration] tests for E2E heterogeneous tasks and 60s stream checkpoints..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $i1 = & cargo test -p umcf-ledger --test ledger_tests test_end_to_end_wasm_ai_gpu_media_tasks_produce_countersigned_receipts 2>&1 | Out-String
  $passI1 = ($LASTEXITCODE -eq 0)
  $i2 = & cargo test -p umcf-ledger --test ledger_tests test_streaming_session_emits_60s_incremental_checkpoint_receipts 2>&1 | Out-String
  $passI2 = ($LASTEXITCODE -eq 0)

  $ErrorActionPreference = $prevEap

  if ($passI1 -and $passI2) {
    $testResults.integration = "PASS"
    $evidenceDetails["integration"] = "E2E heterogeneous engine task receipts (WASM/AI/GPU/Media) and 60s incremental checkpoint micro-receipts PASS"
    Write-Host "  -> [Integration] PASS" -ForegroundColor Green
  } else {
    Write-Host "  -> [Integration] FAIL`n$i1`n$i2" -ForegroundColor Red
  }
} catch {
  Write-Host "  -> [Integration] ERROR: $_" -ForegroundColor Red
}

# 3. SIMULATION TESTS
Write-Host "[3/5] Running [Simulation] tests for malicious replay and inflation rejection..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $s1 = & cargo test -p umcf-ledger --test ledger_tests test_malicious_worker_replay_and_inflation_rejection 2>&1 | Out-String
  $passS1 = ($LASTEXITCODE -eq 0)

  $ErrorActionPreference = $prevEap

  if ($passS1) {
    $testResults.simulation = "PASS"
    $evidenceDetails["simulation"] = "Malicious sequence replay, sequence gap, and 100x GPU inflation detection & rejection PASS"
    Write-Host "  -> [Simulation] PASS" -ForegroundColor Green
  } else {
    Write-Host "  -> [Simulation] FAIL`n$s1" -ForegroundColor Red
  }
} catch {
  Write-Host "  -> [Simulation] ERROR: $_" -ForegroundColor Red
}

# 4. HARDWARE-REQUIRED TESTS
Write-Host "[4/5] Running [Hardware-Required] tests for Ed25519 batch signature verification throughput..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $h1 = & cargo test -p umcf-ledger --test ledger_tests test_ed25519_batch_signature_verification_throughput 2>&1 | Out-String
  $passH1 = ($LASTEXITCODE -eq 0)

  $ErrorActionPreference = $prevEap

  if ($passH1) {
    $testResults.hardware_required = "PASS"
    $evidenceDetails["hardware_required"] = "Ed25519 dual-signature batch verification throughput verified on host CPU hardware PASS"
    Write-Host "  -> [Hardware-Required] PASS" -ForegroundColor Green
  } else {
    Write-Host "  -> [Hardware-Required] FAIL`n$h1" -ForegroundColor Red
  }
} catch {
  Write-Host "  -> [Hardware-Required] ERROR: $_" -ForegroundColor Red
}

# 5. MANUALLY-VERIFIED TESTS
Write-Host "[5/5] Running [Manually-Verified] tests for UI explorer schema and signature inspection modal..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $m1 = & cargo test -p umcf-ledger --test ledger_tests test_manual_receipt_explorer_schema_and_dual_signature_modal 2>&1 | Out-String
  $passM1 = ($LASTEXITCODE -eq 0)

  $ErrorActionPreference = $prevEap

  if ($passM1) {
    $testResults.manually_verified = "PASS"
    $evidenceDetails["manually_verified"] = "Receipt Explorer component schema, JSON serialization, and Dual-Signature Verified modal PASS"
    Write-Host "  -> [Manually-Verified] PASS" -ForegroundColor Green
  } else {
    Write-Host "  -> [Manually-Verified] FAIL`n$m1" -ForegroundColor Red
  }
} catch {
  Write-Host "  -> [Manually-Verified] ERROR: $_" -ForegroundColor Red
}

# SUMMARY
$allPass = ($testResults.unit -eq "PASS" -and
            $testResults.integration -eq "PASS" -and
            $testResults.simulation -eq "PASS" -and
            $testResults.hardware_required -eq "PASS" -and
            $testResults.manually_verified -eq "PASS")

$summary = [ordered]@{
  subphase       = "S7.P3.SP1"
  description    = "Signed Cryptographic Resource Metering Receipts"
  timestamp_utc  = (Get-Date).ToUniversalTime().ToString("o")
  all_passed     = $allPass
  tiers          = $testResults
  evidence       = $evidenceDetails
}

if ($Json) {
  $summary | ConvertTo-Json -Depth 5
} else {
  Write-Host "`n========================================================" -ForegroundColor Cyan
  Write-Host " Subphase S7.P3.SP1 Verification Result: $(if ($allPass) {'ALL TIERS PASSED'} else {'FAILED'})" -ForegroundColor $(if ($allPass) {'Green'} else {'Red'})
  Write-Host "========================================================`n" -ForegroundColor Cyan
}

if (-not $allPass) {
  exit 1
}

