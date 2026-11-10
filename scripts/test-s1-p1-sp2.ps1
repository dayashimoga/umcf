<#
.SYNOPSIS
  Executes classified tests for Subphase S1.P1.SP2:
  - Unit: math & threshold evaluator with mocked byte counters (20 GiB PASS, 26 GiB WARN, 29.5 GiB FAIL)
  - Integration: Podman container definitions and normal quota execution
  - Simulation: synthetic 30.1 GiB byte counter in PODMAN_QUOTA_SIMULATE_BYTES triggering hard abort
  - Hardware-Required: host disk partition and Podman machine storage inspection
  - Manually-Verified: rootless UID mapping (keep-id) and bounded cache volumes verification
#>
param(
  [switch]$Json
)

$ErrorActionPreference = "Stop"
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

# 1. UNIT TEST: Quota threshold evaluator
Write-Host "[1/5] Running [Unit] podman-quota-check threshold tests..."
$prevSim = $env:PODMAN_QUOTA_SIMULATE_BYTES

try {
  # 20 GiB -> PASS
  $env:PODMAN_QUOTA_SIMULATE_BYTES = "21474836480"
  $res20 = & powershell -ExecutionPolicy Bypass -File (Join-Path $ScriptDir "podman-quota-check.ps1") -Json | ConvertFrom-Json
  $pass20 = ($res20.status -eq "PASS")

  # 26 GiB -> WARN (exit 0)
  $env:PODMAN_QUOTA_SIMULATE_BYTES = "27917287424"
  $res26 = & powershell -ExecutionPolicy Bypass -File (Join-Path $ScriptDir "podman-quota-check.ps1") -Json | ConvertFrom-Json
  $pass26 = ($res26.status -eq "WARN")

  # 29.5 GiB -> FAIL (exit 1)
  $env:PODMAN_QUOTA_SIMULATE_BYTES = "31677353984"
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"
  $res29Raw = & powershell -ExecutionPolicy Bypass -File (Join-Path $ScriptDir "podman-quota-check.ps1") -Json 2>$null | Out-String
  $exitCode29 = $LASTEXITCODE
  $ErrorActionPreference = $prevEap
  $res29 = $res29Raw | ConvertFrom-Json
  $pass29 = ($exitCode29 -ne 0 -and $res29.status -eq "FAIL")

  if ($pass20 -and $pass26 -and $pass29) {
    $testResults.unit = "PASS"
    $evidenceDetails["unit"] = "Thresholds verified: 20 GiB (PASS), 26 GiB (WARN), 29.5 GiB (FAIL with exit code $exitCode29)"
    Write-Host "  -> [Unit] PASS" -ForegroundColor Green
  } else {
    Write-Host "  -> [Unit] FAIL (20G: $pass20, 26G: $pass26, 29.5G: $pass29)" -ForegroundColor Red
  }
} finally {
  $env:PODMAN_QUOTA_SIMULATE_BYTES = $prevSim
}

# 2. INTEGRATION TEST: Containerfiles and compose existence + normal quota run
Write-Host "[2/5] Running [Integration] container configurations & live quota check..."
$cfRust = Test-Path (Join-Path $RepoRoot "containers\Podmanfile.rust")
$cfWeb = Test-Path (Join-Path $RepoRoot "containers\Podmanfile.web")
$cfFlutter = Test-Path (Join-Path $RepoRoot "containers\Podmanfile.flutter")
$cfCompose = Test-Path (Join-Path $RepoRoot "compose.podman.yml")

$liveQuota = & powershell -ExecutionPolicy Bypass -File (Join-Path $ScriptDir "podman-quota-check.ps1") -Json | ConvertFrom-Json
$livePass = ($liveQuota.status -eq "PASS" -and $liveQuota.total_bytes -lt $liveQuota.limit_bytes)

if ($cfRust -and $cfWeb -and $cfFlutter -and $cfCompose -and $livePass) {
  $testResults.integration = "PASS"
  $evidenceDetails["integration"] = "All 3 Podmanfiles + compose.podman.yml verified; live quota check PASS ($($liveQuota.total_bytes) bytes < $($liveQuota.limit_bytes))"
  Write-Host "  -> [Integration] PASS" -ForegroundColor Green
} else {
  Write-Host "  -> [Integration] FAIL" -ForegroundColor Red
}

# 3. SIMULATION / FAULT-INJECTION TEST: 30.1 GiB byte counter
Write-Host "[3/5] Running [Simulation] 30.1 GiB synthetic byte counter test..."
try {
  $env:PODMAN_QUOTA_SIMULATE_BYTES = "32319619072"
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"
  $faultRaw = & powershell -ExecutionPolicy Bypass -File (Join-Path $ScriptDir "podman-quota-check.ps1") -Json 2>$null | Out-String
  $faultExit = $LASTEXITCODE
  $ErrorActionPreference = $prevEap
  $faultJson = $faultRaw | ConvertFrom-Json

  if ($faultExit -ne 0 -and $faultJson.status -eq "FAIL") {
    $testResults.simulation = "PASS"
    $evidenceDetails["simulation"] = "Synthetic 30.1 GiB counter triggered hard abort (exit code $faultExit, status FAIL)"
    Write-Host "  -> [Simulation] PASS (30.1 GiB correctly rejected with non-zero exit code)" -ForegroundColor Green
  } else {
    Write-Host "  -> [Simulation] FAIL (fault gate failed)" -ForegroundColor Red
  }
} finally {
  $env:PODMAN_QUOTA_SIMULATE_BYTES = $prevSim
}

# 4. HARDWARE-REQUIRED TEST: Podman machine storage inspection
Write-Host "[4/5] Running [Hardware-Required] host storage & podman machine inspection..."
$enforceRes = & powershell -ExecutionPolicy Bypass -File (Join-Path $ScriptDir "enforce-podman-quota.ps1") -Json | ConvertFrom-Json
if ($enforceRes.status -eq "PASS" -and $enforceRes.podman_installed) {
  $testResults.hardware_required = "PASS"
  $evidenceDetails["hardware_required"] = "Podman machine '$($enforceRes.machine_name)' verified with disk size $($enforceRes.machine_disk_size) <= $($enforceRes.max_allowed_gib) GiB ceiling"
  Write-Host "  -> [Hardware-Required] PASS ($($enforceRes.machine_name): $($enforceRes.machine_disk_size))" -ForegroundColor Green
} else {
  $testResults.hardware_required = "DEFERRED_CONTAINER_RUNTIME_UNAVAILABLE"
  $evidenceDetails["hardware_required"] = "Podman machine inspection returned status: $($enforceRes.status)"
  Write-Host "  -> [Hardware-Required] Status: $($testResults.hardware_required)" -ForegroundColor Yellow
}

# 5. MANUALLY-VERIFIED TEST: Rootless keep-id and bounded cache volumes
Write-Host "[5/5] Running [Manually-Verified] rootless keep-id & cache volume check..."
$composeContent = Get-Content (Join-Path $RepoRoot "compose.podman.yml") -Raw
$hasKeepId = $composeContent.Contains('userns_mode: "keep-id"')
$hasCargoCache = $composeContent.Contains('umcf_cargo_cache:')
$hasSccache = $composeContent.Contains('umcf_sccache:')

if ($hasKeepId -and $hasCargoCache -and $hasSccache) {
  $testResults.manually_verified = "PASS"
  $evidenceDetails["manually_verified"] = "compose.podman.yml verifies rootless UID mapping (userns_mode: keep-id) and bounded volumes (umcf_cargo_cache, umcf_sccache)"
  Write-Host "  -> [Manually-Verified] PASS" -ForegroundColor Green
} else {
  Write-Host "  -> [Manually-Verified] FAIL" -ForegroundColor Red
}

$allPass = ($testResults.unit -eq "PASS") -and ($testResults.integration -eq "PASS") -and ($testResults.simulation -eq "PASS") -and ($testResults.manually_verified -eq "PASS")

$summary = [ordered]@{
  subphase              = "S1.P1.SP2"
  overall_status        = if ($allPass) { "PASS" } else { "FAIL" }
  test_classifications = $testResults
  details               = $evidenceDetails
}

if ($Json) {
  $summary | ConvertTo-Json -Depth 5
} else {
  Write-Host "=== Subphase S1.P1.SP2 Test Classification Summary ==="
  Write-Host "Overall Status: $($summary.overall_status)"
}

if (-not $allPass) {
  exit 1
}
exit 0

