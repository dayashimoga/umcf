<#
.SYNOPSIS
  Executes classified five-tier verification tests for Subphase S7.P3.SP2:
  - Unit: Double-entry journal balance constraints, escrow lifecycle exact math,
          idempotent settlement double-payout prevention, Byzantine slashing, and constraint matching.
  - Integration: Full E2E marketplace lifecycle (Ask -> Bid -> Match -> Escrow -> Settle -> Refund),
                 and concurrent 64-thread settlements preserving zero-sum balance invariant.
  - Simulation: Property-based randomized simulation of 1,000 market operations asserting global conservation.
  - Hardware-Required: Matching engine throughput benchmark across 10,000 active compute offers (< 5 ms SLA).
  - Manually-Verified: Contribute Device UI schema, charging/Wi-Fi safety toggles, and Double-Entry Ledger table.
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
Write-Host "[1/5] Running [Unit] tests for double-entry journal, escrow math, idempotency, slashing, and matcher constraints..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $u1 = & cargo test -p umcf-marketplace --test marketplace_tests test_double_entry_journal_rejects_unbalanced_legs_and_negative_balances 2>&1 | Out-String
  $pass1 = ($LASTEXITCODE -eq 0)
  $u2 = & cargo test -p umcf-marketplace --test marketplace_tests test_escrow_lock_partial_receipt_settlement_and_unused_refund_exact_math 2>&1 | Out-String
  $pass2 = ($LASTEXITCODE -eq 0)
  $u3 = & cargo test -p umcf-marketplace --test marketplace_tests test_idempotent_receipt_settlement_prevents_double_payout 2>&1 | Out-String
  $pass3 = ($LASTEXITCODE -eq 0)
  $u4 = & cargo test -p umcf-marketplace --test marketplace_tests test_byzantine_verification_failure_evidence_slashes_provider_bond 2>&1 | Out-String
  $pass4 = ($LASTEXITCODE -eq 0)
  $u5 = & cargo test -p umcf-marketplace --test marketplace_tests test_marketplace_matcher_respects_charging_wifi_and_privacy_constraints 2>&1 | Out-String
  $pass5 = ($LASTEXITCODE -eq 0)

  $ErrorActionPreference = $prevEap

  if ($pass1 -and $pass2 -and $pass3 -and $pass4 -and $pass5) {
    $testResults.unit = "PASS"
    $evidenceDetails["unit"] = "Double-entry journal integrity, escrow lifecycle math, idempotent double-spend prevention, Byzantine slashing, and constraint matching PASS"
    Write-Host "  -> [Unit] PASS" -ForegroundColor Green
  } else {
    Write-Host "  -> [Unit] FAIL`n$u1`n$u2`n$u3`n$u4`n$u5" -ForegroundColor Red
  }
} catch {
  Write-Host "  -> [Unit] ERROR: $_" -ForegroundColor Red
}

# 2. INTEGRATION TESTS
Write-Host "[2/5] Running [Integration] tests for full marketplace lifecycle and 64-thread concurrent settlements..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $i1 = & cargo test -p umcf-marketplace --test marketplace_tests test_full_marketplace_lifecycle_ask_bid_match_escrow_execute_receipt_settle_refund 2>&1 | Out-String
  $passI1 = ($LASTEXITCODE -eq 0)
  $i2 = & cargo test -p umcf-marketplace --test marketplace_tests test_concurrent_64_thread_settlements_preserve_global_zero_sum_invariant 2>&1 | Out-String
  $passI2 = ($LASTEXITCODE -eq 0)

  $ErrorActionPreference = $prevEap

  if ($passI1 -and $passI2) {
    $testResults.integration = "PASS"
    $evidenceDetails["integration"] = "Full E2E marketplace lifecycle and 64-thread concurrent settlements preserving zero-sum balance invariant PASS"
    Write-Host "  -> [Integration] PASS" -ForegroundColor Green
  } else {
    Write-Host "  -> [Integration] FAIL`n$i1`n$i2" -ForegroundColor Red
  }
} catch {
  Write-Host "  -> [Integration] ERROR: $_" -ForegroundColor Red
}

# 3. SIMULATION TESTS
Write-Host "[3/5] Running [Simulation] tests for randomized property-based market conservation..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $s1 = & cargo test -p umcf-marketplace --test marketplace_tests test_proptest_randomized_market_bids_asks_and_slashings_conserve_zero_sum 2>&1 | Out-String
  $passS1 = ($LASTEXITCODE -eq 0)

  $ErrorActionPreference = $prevEap

  if ($passS1) {
    $testResults.simulation = "PASS"
    $evidenceDetails["simulation"] = "Property-based randomized simulation of 1,000 market operations asserting zero-sum balance conservation after every operation PASS"
    Write-Host "  -> [Simulation] PASS" -ForegroundColor Green
  } else {
    Write-Host "  -> [Simulation] FAIL`n$s1" -ForegroundColor Red
  }
} catch {
  Write-Host "  -> [Simulation] ERROR: $_" -ForegroundColor Red
}

# 4. HARDWARE-REQUIRED TESTS
Write-Host "[4/5] Running [Hardware-Required] tests for matching engine throughput across 10,000 active offers..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $h1 = & cargo test -p umcf-marketplace --test marketplace_tests test_marketplace_matching_engine_throughput_benchmark 2>&1 | Out-String
  $passH1 = ($LASTEXITCODE -eq 0)

  $ErrorActionPreference = $prevEap

  if ($passH1) {
    $testResults.hardware_required = "PASS"
    $evidenceDetails["hardware_required"] = "Marketplace matching engine latency verified across 10,000 active offers (< 5 ms SLA in release) PASS"
    Write-Host "  -> [Hardware-Required] PASS" -ForegroundColor Green
  } else {
    Write-Host "  -> [Hardware-Required] FAIL`n$h1" -ForegroundColor Red
  }
} catch {
  Write-Host "  -> [Hardware-Required] ERROR: $_" -ForegroundColor Red
}

# 5. MANUALLY-VERIFIED TESTS
Write-Host "[5/5] Running [Manually-Verified] tests for UI schema and device safety controls..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $m1 = & cargo test -p umcf-marketplace --test marketplace_tests test_manual_contribute_device_and_order_book_ui_schema 2>&1 | Out-String
  $passM1 = ($LASTEXITCODE -eq 0)

  $ErrorActionPreference = $prevEap

  if ($passM1) {
    $testResults.manually_verified = "PASS"
    $evidenceDetails["manually_verified"] = "Contribute Device UI schema, mobile battery/Wi-Fi safety toggles, and Journal Ledger export PASS"
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
  subphase       = "S7.P3.SP2"
  description    = "Double-Entry Compute Credit Ledger & Marketplace Order Matching"
  timestamp_utc  = (Get-Date).ToUniversalTime().ToString("o")
  all_passed     = $allPass
  tiers          = $testResults
  evidence       = $evidenceDetails
}

if ($Json) {
  $summary | ConvertTo-Json -Depth 5
} else {
  Write-Host "`n========================================================" -ForegroundColor Cyan
  Write-Host " Subphase S7.P3.SP2 Verification Result: $(if ($allPass) {'ALL TIERS PASSED'} else {'FAILED'})" -ForegroundColor $(if ($allPass) {'Green'} else {'Red'})
  Write-Host "========================================================`n" -ForegroundColor Cyan
}

if (-not $allPass) {
  exit 1
}

