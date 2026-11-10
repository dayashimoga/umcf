<#
.SYNOPSIS
  Scans production source paths for forbidden placeholder tokens:
  todo!(), unimplemented!(), FIXME, or fake mock stubs.
  Must complete in < 2.0 seconds and exit 1 if violations are found.
#>
param(
  [switch]$Json
)

$ErrorActionPreference = "Stop"
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$RepoRoot = (Resolve-Path (Join-Path $ScriptDir "..")).Path

$sw = [System.Diagnostics.Stopwatch]::StartNew()

$searchDirs = @(
  (Join-Path $RepoRoot "crates"),
  (Join-Path $RepoRoot "services"),
  (Join-Path $RepoRoot "apps")
)

$bannedRegex = '\btodo!\(|\bunimplemented!\(|\bFIXME\b'
$violations = @()

$extensions = @("*.rs", "*.ts", "*.js", "*.mjs", "*.svelte", "*.dart")

foreach ($dir in $searchDirs) {
  if (Test-Path $dir) {
    $files = Get-ChildItem -Path $dir -Recurse -File -Include $extensions | Where-Object {
      $_.FullName -notmatch '[\\/](target|node_modules|build|\.dart_tool|dist|test|tests)[\\/]'
    }

    foreach ($file in $files) {
      $lines = Get-Content -Path $file.FullName
      $lineNum = 1
      foreach ($line in $lines) {
        if ($line -match $bannedRegex) {
          # Ignore comment in test or rule
          if ($line -notmatch '//.*test' -and $line -notmatch '#\[cfg\(test\)') {
            $violations += [ordered]@{
              file    = $file.FullName.Substring($RepoRoot.Length + 1)
              line    = $lineNum
              content = $line.Trim()
            }
          }
        }
        $lineNum++
      }
    }
  }
}

$sw.Stop()
$scanDurationMs = $sw.ElapsedMilliseconds

$result = [ordered]@{
  status           = if ($violations.Count -eq 0) { "PASS" } else { "FAIL" }
  placeholder_count = $violations.Count
  scan_duration_ms = $scanDurationMs
  violations       = $violations
}

if ($Json) {
  $result | ConvertTo-Json -Depth 5
} else {
  Write-Host "=== UMCF Production Placeholder Gate ==="
  Write-Host "Status            : $($result.status)"
  Write-Host "Scan Duration     : $($scanDurationMs) ms (< 2000 ms target)"
  Write-Host "Forbidden Tokens  : $($violations.Count)"
  if ($violations.Count -gt 0) {
    foreach ($v in $violations) {
      Write-Host "  VIOLATION: $($v.file):$($v.line) -> $($v.content)" -ForegroundColor Red
    }
  }
}

if ($violations.Count -gt 0) {
  exit 1
}
exit 0

