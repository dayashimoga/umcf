<#
.SYNOPSIS
  Executes classified five-tier verification tests for Subphase S7.P2.SP1:
  - Unit: API key scoped capability token enforcement & token-bucket rate limiter,
          RFC 9457 Problem Details structured error serialization,
          OpenAPI 3.1.0 schema completeness and validity,
          and WebSocket event ring buffer monotonic sequence cursor replay.
  - Integration: Rust, TypeScript, and Python SDK end-to-end conformance suite
                 (cluster, WASM jobs, AI chat completion, and FastCDC storage roundtrip).
  - Simulation: Network disconnection & lossless since_seq event stream resumption.
  - Hardware-Required: Gateway loopback routing latency benchmark (< 0.5 ms P99 SLA).
  - Manually-Verified: Interactive API docs explorer HTML & ApiKeyManager UI schema.
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
Write-Host "[1/5] Running [Unit] tests for scoped tokens, rate limiting, RFC 9457, OpenAPI 3.1, and ring buffer..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $u1 = & cargo test -p umcf-api-gateway --test gateway_tests test_api_key_capability_scope_and_rate_limiter_enforcement 2>&1 | Out-String
  $pass1 = ($LASTEXITCODE -eq 0)
  $u2 = & cargo test -p umcf-api-gateway --test gateway_tests test_rfc9457_problem_details_error_serialization 2>&1 | Out-String
  $pass2 = ($LASTEXITCODE -eq 0)
  $u3 = & cargo test -p umcf-api-gateway --test gateway_tests test_openapi_3_1_schema_completeness_and_validity 2>&1 | Out-String
  $pass3 = ($LASTEXITCODE -eq 0)
  $u4 = & cargo test -p umcf-api-gateway --test gateway_tests test_websocket_event_ring_buffer_cursor_replay 2>&1 | Out-String
  $pass4 = ($LASTEXITCODE -eq 0)

  $ErrorActionPreference = $prevEap

  if ($pass1 -and $pass2 -and $pass3 -and $pass4) {
    $testResults.unit = "PASS"
    $evidenceDetails["unit"] = "Scoped capability token validation, rate-limiting token bucket, RFC 9457 Problem Details, OpenAPI 3.1 spec, and WebSocket ring buffer replay PASS"
    Write-Host "  -> [Unit] PASS" -ForegroundColor Green
  } else {
    Write-Host "  -> [Unit] FAIL`n$u1`n$u2`n$u3`n$u4" -ForegroundColor Red
  }
} catch {
  Write-Host "  -> [Unit] ERROR: $_" -ForegroundColor Red
}

# 2. INTEGRATION TESTS
Write-Host "[2/5] Running [Integration] tests for Rust, TypeScript, and Python SDKs..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $i1 = & cargo test -p umcf-api-gateway --test gateway_tests test_rust_sdk_e2e_wasm_job_ai_stream_and_storage_roundtrip 2>&1 | Out-String
  $passI1 = ($LASTEXITCODE -eq 0)
  $i2 = & cargo test -p umcf-api-gateway --test gateway_tests test_typescript_sdk_e2e_conformance_suite 2>&1 | Out-String
  $passI2 = ($LASTEXITCODE -eq 0)
  $i3 = & cargo test -p umcf-api-gateway --test gateway_tests test_python_sdk_sync_and_async_e2e_conformance_suite 2>&1 | Out-String
  $passI3 = ($LASTEXITCODE -eq 0)

  $ErrorActionPreference = $prevEap

  if ($passI1 -and $passI2 -and $passI3) {
    $testResults.integration = "PASS"
    $evidenceDetails["integration"] = "Rust SDK, TypeScript SDK, and Python SDK polyglot conformance across cluster, jobs, AI, and storage PASS"
    Write-Host "  -> [Integration] PASS" -ForegroundColor Green
  } else {
    Write-Host "  -> [Integration] FAIL`n$i1`n$i2`n$i3" -ForegroundColor Red
  }
} catch {
  Write-Host "  -> [Integration] ERROR: $_" -ForegroundColor Red
}

# 3. SIMULATION TESTS
Write-Host "[3/5] Running [Simulation] test for connection resets and lossless sequence cursor resumption..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $simOut = & cargo test -p umcf-api-gateway --test gateway_tests test_reconnection_and_lossless_since_seq_event_resumption 2>&1 | Out-String
  $simPass = ($LASTEXITCODE -eq 0)

  $ErrorActionPreference = $prevEap

  if ($simPass) {
    $testResults.simulation = "PASS"
    $evidenceDetails["simulation"] = "Event stream reconnection with since_seq cursor replay across missed events with zero gaps or duplicates PASS"
    Write-Host "  -> [Simulation] PASS" -ForegroundColor Green
  } else {
    Write-Host "  -> [Simulation] FAIL`n$simOut" -ForegroundColor Red
  }
} catch {
  Write-Host "  -> [Simulation] ERROR: $_" -ForegroundColor Red
}

# 4. HARDWARE-REQUIRED TESTS
Write-Host "[4/5] Running [Hardware-Required] benchmark for gateway loopback routing latency..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $hwOut = & cargo test -p umcf-api-gateway --test gateway_tests test_hardware_gateway_loopback_routing_latency_benchmark -- --nocapture 2>&1 | Out-String
  $hwPass = ($LASTEXITCODE -eq 0)

  $ErrorActionPreference = $prevEap

  if ($hwPass) {
    $testResults.hardware_required = "PASS"
    $evidenceDetails["hardware_required"] = "API Gateway loopback routing overhead P99 latency benchmark PASS"
    Write-Host "  -> [Hardware-Required] PASS" -ForegroundColor Green
  } else {
    Write-Host "  -> [Hardware-Required] FAIL`n$hwOut" -ForegroundColor Red
  }
} catch {
  Write-Host "  -> [Hardware-Required] ERROR: $_" -ForegroundColor Red
}

# 5. MANUALLY-VERIFIED TESTS
Write-Host "[5/5] Running [Manually-Verified] test for API docs explorer HTML and ApiKeyManager schema..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $manOut = & cargo test -p umcf-api-gateway --test gateway_tests test_manual_api_docs_and_key_manager_ui_schema 2>&1 | Out-String
  $manPass = ($LASTEXITCODE -eq 0)

  $ErrorActionPreference = $prevEap

  if ($manPass) {
    $testResults.manually_verified = "PASS"
    $evidenceDetails["manually_verified"] = "Interactive API docs explorer HTML generation, OpenAPI link, and ApiKeyManager UI schema PASS"
    Write-Host "  -> [Manually-Verified] PASS" -ForegroundColor Green
  } else {
    Write-Host "  -> [Manually-Verified] FAIL`n$manOut" -ForegroundColor Red
  }
} catch {
  Write-Host "  -> [Manually-Verified] ERROR: $_" -ForegroundColor Red
}

# SUMMARY
Write-Host "`n========================================================" -ForegroundColor Cyan
Write-Host "  S7.P2.SP1 Five-Tier Verification Summary" -ForegroundColor Cyan
Write-Host "========================================================" -ForegroundColor Cyan
$allPass = $true
foreach ($tier in $testResults.Keys) {
  $status = $testResults[$tier]
  $color = if ($status -eq "PASS") { "Green" } else { "Red"; $allPass = $false }
  Write-Host ("  {0,-20}: {1}" -f $tier.ToUpper(), $status) -ForegroundColor $color
}
Write-Host "========================================================`n" -ForegroundColor Cyan

if ($Json) {
  $outputObj = [ordered]@{
    subphase          = "S7.P2.SP1"
    timestamp         = (Get-Date).ToString("o")
    results           = $testResults
    evidence          = $evidenceDetails
    overall_status    = if ($allPass) { "PASS" } else { "FAIL" }
  }
  $outputObj | ConvertTo-Json -Depth 4
}

if (-not $allPass) {
  exit 1
}
exit 0

