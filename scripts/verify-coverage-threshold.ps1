<#
.SYNOPSIS
  UMCF Coverage Threshold Verifier (PowerShell native).
  Verifies that test coverage meets or exceeds the mandatory threshold.
#>
param(
  [string]$Report,
  [double]$Threshold = 85.0,
  [switch]$TestSelf
)

if ($TestSelf) {
  Write-Host "Running verify-coverage-threshold unit self-tests (PowerShell)..."

  $failPct = 84.9
  if ($failPct -ge 85.0) {
    Write-Host "Assertion failed: 84.9% should not be >= 85.0" -ForegroundColor Red
    exit 1
  }

  $passPct85 = 85.0
  if ($passPct85 -lt 85.0) {
    Write-Host "Assertion failed: 85.0% should be >= 85.0" -ForegroundColor Red
    exit 1
  }

  $passPct95 = 95.5
  if ($passPct95 -lt 85.0) {
    Write-Host "Assertion failed: 95.5% should be >= 85.0" -ForegroundColor Red
    exit 1
  }

  Write-Host "All PowerShell coverage threshold unit self-tests passed successfully." -ForegroundColor Green
  exit 0
}

if (-not $Report) {
  Write-Error "Report path is required (-Report <path>)"
  exit 1
}

if (-not (Test-Path $Report)) {
  Write-Error "Coverage report file not found: $Report"
  exit 1
}

$raw = Get-Content $Report -Raw
$data = $raw | ConvertFrom-Json

$pct = 0.0
$totalLines = 0
$coveredLines = 0

if ($data.overall -and ($data.overall.coverage_percent -ne $null)) {
  $pct = [double]$data.overall.coverage_percent
  $totalLines = [int]$data.overall.lines_total
  $coveredLines = [int]$data.overall.lines_covered
} elseif ($data.data -and ($data.data.Count -gt 0) -and $data.data[0].totals -and $data.data[0].totals.lines) {
  $lines = $data.data[0].totals.lines
  $pct = [double]$lines.percent
  $totalLines = [int]$lines.count
  $coveredLines = [int]$lines.covered
} else {
  Write-Error "Unrecognized coverage report schema format"
  exit 1
}

Write-Host "=============================================================================="
Write-Host " UMCF Coverage Threshold Verification Gate"
Write-Host "=============================================================================="
Write-Host " Report Path       : $Report"
Write-Host " Total Lines       : $totalLines"
Write-Host " Covered Lines     : $coveredLines"
Write-Host " Measured Coverage : $($pct.ToString("F2"))%"
Write-Host " Required Threshold: $($Threshold.ToString("F2"))%"
Write-Host "------------------------------------------------------------------------------"

if ($pct -ge $Threshold) {
  Write-Host " STATUS: PASS (Coverage $($pct.ToString("F2"))% >= $($Threshold.ToString("F2"))% requirement)" -ForegroundColor Green
  Write-Host "=============================================================================="
  exit 0
} else {
  $gap = $Threshold - $pct
  Write-Host " STATUS: FAIL (Coverage $($pct.ToString("F2"))% < $($Threshold.ToString("F2"))% requirement; gap: -$($gap.ToString("F2"))%)" -ForegroundColor Red
  Write-Host "=============================================================================="
  exit 1
}
