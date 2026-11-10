<#
.SYNOPSIS
  Executes classified tests for Subphase S3.P3.SP1:
  - Unit: /proc/cpuinfo, cgroup v2 cpu.max/memory.max/memory.current, battery sysfs, and thermal mapping parsers
  - Integration: SystemProber::probe_all() live hardware probe (< 25 ms, populated descriptor)
  - Simulation: Thermal critical (95,000 mC) and low battery (8% discharging) sysfs injection, cgroup limits clamping
  - Hardware-Required: Real CPUID SIMD feature detection, host RAM bounds, and Podman storage quota check (< 29.0 GiB)
  - Manually-Verified: live_system_probe_report.json audit
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
Write-Host "[1/5] Running [Unit] tests for CPU/RAM/Battery/Thermal parsers..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $unitOut = & cargo test -p umcf-hwprobe --test hwprobe_tests test_unit_ 2>&1 | Out-String
  $pass = ($LASTEXITCODE -eq 0)

  $ErrorActionPreference = $prevEap

  if ($pass) {
    $testResults.unit = "PASS"
    $evidenceDetails["unit"] = "Parsers for /proc/cpuinfo (x86_64, ARM64 Android), cgroup v2 cpu.max/memory.max/memory.current, battery sysfs, and thermal temperature mapping PASS"
    Write-Host "  -> [Unit] PASS" -ForegroundColor Green
  } else {
    Write-Host "  -> [Unit] FAIL`n$unitOut" -ForegroundColor Red
  }
} catch {
  Write-Host "  -> [Unit] ERROR: $_" -ForegroundColor Red
}

# 2. INTEGRATION TESTS
Write-Host "[2/5] Running [Integration] SystemProber::probe_all() live query (< 25 ms, populated descriptor)..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $intOut = & cargo test -p umcf-hwprobe --release --test hwprobe_tests test_integration_system_probe_live -- --nocapture 2>&1 | Out-String
  $intPass = ($LASTEXITCODE -eq 0)

  $ErrorActionPreference = $prevEap

  if ($intPass) {
    $testResults.integration = "PASS"
    $evidenceDetails["integration"] = "Live SystemProber::probe_all() completed in < 25 ms, returning non-zero cores, valid RAM/storage bounds, and populated descriptor PASS"
    Write-Host "  -> [Integration] PASS" -ForegroundColor Green
  } else {
    Write-Host "  -> [Integration] FAIL`n$intOut" -ForegroundColor Red
  }
} catch {
  Write-Host "  -> [Integration] ERROR: $_" -ForegroundColor Red
}

# 3. SIMULATION / FAULT-INJECTION TESTS
Write-Host "[3/5] Running [Simulation] thermal critical (95,000 mC), low battery (8%), and cgroup limit clamping..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $simOut = & cargo test -p umcf-hwprobe --test hwprobe_tests test_simulation_ 2>&1 | Out-String
  $simPass = ($LASTEXITCODE -eq 0)

  $ErrorActionPreference = $prevEap

  if ($simPass) {
    $testResults.simulation = "PASS"
    $evidenceDetails["simulation"] = "Synthetic sysfs thermal critical (95°C -> ThermalState::Critical), low battery (8% discharging), and cgroup container memory clamping PASS"
    Write-Host "  -> [Simulation] PASS" -ForegroundColor Green
  } else {
    Write-Host "  -> [Simulation] FAIL`n$simOut" -ForegroundColor Red
  }
} catch {
  Write-Host "  -> [Simulation] ERROR: $_" -ForegroundColor Red
}

# 4. HARDWARE-REQUIRED TESTS
Write-Host "[4/5] Running [Hardware-Required] CPUID SIMD register queries, physical RAM bounds, and Podman quota..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $hwOut = & cargo test -p umcf-hwprobe --test hwprobe_tests test_hw_required_ -- --nocapture 2>&1 | Out-String
  $hwPass = ($LASTEXITCODE -eq 0)

  $quotaOutput = & powershell -ExecutionPolicy Bypass -File (Join-Path $ScriptDir "podman-quota-check.ps1") 2>&1 | Out-String
  $quotaPass = ($LASTEXITCODE -eq 0)

  $ErrorActionPreference = $prevEap

  if ($hwPass -and $quotaPass) {
    $testResults.hardware_required = "PASS"
    $evidenceDetails["hardware_required"] = "Physical CPUID SIMD flags matched CpuCapability, host RAM verified >= 1 GB; Podman storage quota compliance PASS (< 29.0 GiB)"
    Write-Host "  -> [Hardware-Required] PASS" -ForegroundColor Green
  } else {
    Write-Host "  -> [Hardware-Required] FAIL (HW: $hwPass, Quota: $quotaPass)" -ForegroundColor Red
  }
} catch {
  Write-Host "  -> [Hardware-Required] ERROR: $_" -ForegroundColor Red
}

# 5. MANUALLY-VERIFIED TESTS
Write-Host "[5/5] Running [Manually-Verified] live_system_probe_report.json audit..."
try {
  $prevEap = $ErrorActionPreference
  $ErrorActionPreference = "Continue"

  $auditOut = & cargo test -p umcf-hwprobe --test hwprobe_tests test_manually_verified_ 2>&1 | Out-String
  $auditPass = ($LASTEXITCODE -eq 0)

  $ErrorActionPreference = $prevEap

  $reportPath = Join-Path $RepoRoot "artifacts/evidence/stage-03/live_system_probe_report.json"
  if ($auditPass -and (Test-Path $reportPath)) {
    $report = Get-Content $reportPath -Raw | ConvertFrom-Json

    $nodeIdValid = ($report.node_id -like "did:umcf:*")
    $cpuValid = ($report.cpu.physical_cores -ge 1)
    $ramValid = ($report.memory.total_ram -gt 0)
    $diskValid = ($report.storage.total_disk -gt 0)
    $osValid = ($report.os.Length -gt 0)

    if ($nodeIdValid -and $cpuValid -and $ramValid -and $diskValid -and $osValid) {
      $testResults.manually_verified = "PASS"
      $evidenceDetails["manually_verified"] = "live_system_probe_report.json verified: node_id=$($report.node_id), OS=$($report.os), arch=$($report.cpu.architecture), cores=$($report.cpu.physical_cores), RAM=$([math]::Round($report.memory.total_ram/1GB, 2)) GB, disk=$([math]::Round($report.storage.total_disk/1GB, 2)) GB PASS"
      Write-Host "  -> [Manually-Verified] PASS" -ForegroundColor Green
    } else {
      Write-Host "  -> [Manually-Verified] FAIL (Report invalid: CPU cores=$($report.cpu.physical_cores), RAM=$($report.memory.total_ram))" -ForegroundColor Red
    }
  } else {
    Write-Host "  -> [Manually-Verified] FAIL (Audit test failed or live_system_probe_report.json missing)`n$auditOut" -ForegroundColor Red
  }
} catch {
  Write-Host "  -> [Manually-Verified] ERROR: $_" -ForegroundColor Red
}

Write-Host "`nTest Summary for Subphase S3.P3.SP1:"
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
    subphase = "S3.P3.SP1"
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

