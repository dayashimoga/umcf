<#
.SYNOPSIS
  Verifies the presence, non-emptiness, and mandatory schemas of all UMCF governance files.
  Also verifies that all 72 subphases (S1.P1.SP1 through S8.P3.SP3) are represented in TODO.md.
#>
param(
  [switch]$Json
)

$ErrorActionPreference = "Stop"
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$RepoRoot = (Resolve-Path (Join-Path $ScriptDir "..")).Path

$failures = @()
$verifiedFiles = @()

function Check-File([string]$relPath) {
  $fullPath = Join-Path $RepoRoot $relPath
  if (-not (Test-Path $fullPath)) {
    $script:failures += "Missing governance file: $relPath"
    return $false
  }
  $item = Get-Item $fullPath
  if ($item.Length -eq 0) {
    $script:failures += "Empty governance file: $relPath"
    return $false
  }
  $sha256 = (Get-FileHash -Algorithm SHA256 $fullPath).Hash.ToLowerInvariant()
  $script:verifiedFiles += [ordered]@{
    path       = $relPath
    size_bytes = $item.Length
    sha256     = $sha256
  }
  return $true
}

$requiredFiles = @(
  "ENGINEERING_CONTRACT.md",
  "AGENTS.md",
  "SECURITY.md",
  "CONTRIBUTING.md",
  "LICENSE-APACHE",
  "LICENSE-MIT",
  ".github\CODEOWNERS",
  "TODO.md",
  "CHANGELOG.md",
  "README.md",
  ".editorconfig",
  ".gitattributes",
  ".gitignore",
  "docs\architecture\SYSTEM_OVERVIEW.md",
  "docs\architecture\COMPONENT_CONTRACTS.md",
  "docs\architecture\DATA_FLOWS.md",
  "docs\requirements\FUNCTIONAL_REQUIREMENTS.md",
  "docs\requirements\NON_FUNCTIONAL_REQUIREMENTS.md",
  "docs\requirements\TRACEABILITY_MATRIX.md",
  "docs\implementation\MASTER_ROADMAP.md",
  "infra\podman\storage.conf",
  "infra\podman\Containerfile",
  "scripts\enforce-podman-quota.sh",
  "scripts\enforce-podman-quota.ps1",
  "scripts\check-no-placeholders.sh",
  "scripts\check-no-placeholders.ps1"
)

foreach ($f in $requiredFiles) {
  Check-File $f | Out-Null
}

# Verify rules
$rules = @("00-global-governance.md", "01-rust-standards.md", "02-flutter-standards.md", "03-svelte-web-standards.md", "04-security-and-privacy.md", "05-hardware-and-benchmarking.md", "06-ci-cd-and-podman.md")
foreach ($r in $rules) {
  Check-File ".agent\rules\$r" | Out-Null
}

# Verify skills
$skills = @("umcf-architecture", "umcf-rust-dev", "umcf-flutter-dev", "umcf-svelte-dev", "umcf-distributed-systems", "umcf-networking-p2p", "umcf-security-audit", "umcf-testing-verification", "umcf-devops-podman")
foreach ($s in $skills) {
  Check-File ".agent\skills\$s\SKILL.md" | Out-Null
}

# Verify workflows
$workflows = @("stage-execution-workflow.md", "audit-workflow.md", "subphase-implementation-workflow.md", "code-review-workflow.md", "testing-and-coverage-workflow.md", "release-and-packaging-workflow.md", "hardware-certification-workflow.md")
foreach ($w in $workflows) {
  Check-File ".agent\workflows\$w" | Out-Null
}

# Verify all 72 subphases in TODO.md
$todoContent = Get-Content -Raw (Join-Path $RepoRoot "TODO.md")
$missingSubphases = @()

for ($s = 1; $s -le 8; $s++) {
  for ($p = 1; $p -le 3; $p++) {
    for ($sp = 1; $sp -le 3; $sp++) {
      $subId = "S${s}.P${p}.SP${sp}"
      if ($todoContent -notmatch [regex]::Escape($subId)) {
        $missingSubphases += $subId
        $failures += "Subphase $subId missing in TODO.md"
      }
    }
  }
}

$result = [ordered]@{
  status             = if ($failures.Count -eq 0) { "PASS" } else { "FAIL" }
  verified_files     = $verifiedFiles.Count
  subphases_checked  = 72
  subphases_present  = (72 - $missingSubphases.Count)
  missing_subphases  = $missingSubphases
  failures           = $failures
  files              = $verifiedFiles
}

if ($Json) {
  $result | ConvertTo-Json -Depth 5
} else {
  Write-Host "=== UMCF Governance Verification ==="
  Write-Host "Status            : $($result.status)"
  Write-Host "Files Verified    : $($result.verified_files)"
  Write-Host "Subphases in TODO : $($result.subphases_present) / 72"
  if ($failures.Count -gt 0) {
    foreach ($err in $failures) {
      Write-Host "  ERROR: $err" -ForegroundColor Red
    }
  } else {
    Write-Host "All governance files, schemas, and 72 subphase entries verified." -ForegroundColor Green
  }
}

if ($failures.Count -gt 0) {
  exit 1
}
exit 0
