<#
.SYNOPSIS
  Packages standalone build artifacts, generates SHA-256 checksums, build metadata, and SPDX/CycloneDX SBOM manifests.
#>
param(
  [string]$OutputDir = "artifacts"
)

$ErrorActionPreference = "Stop"
$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$TargetOut = Join-Path $RepoRoot $OutputDir

New-Item -ItemType Directory -Force -Path $TargetOut | Out-Null

$prevEap = $ErrorActionPreference
$ErrorActionPreference = "Continue"
$gitCommit = (& git -C $RepoRoot rev-parse HEAD 2>$null) | Out-String
$ErrorActionPreference = $prevEap
$gitCommit = $gitCommit.Trim()
if ([string]::IsNullOrWhiteSpace($gitCommit) -or $LASTEXITCODE -ne 0) {
  $gitCommit = "uncommitted-bootstrap"
}
$timestamp = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ssZ")

$rustBinaries = @(
  @{ Name = "umcf"; Path = (Join-Path $RepoRoot "target\debug\umcf.exe"); ReleasePath = (Join-Path $RepoRoot "target\release\umcf.exe") },
  @{ Name = "umcfd"; Path = (Join-Path $RepoRoot "target\debug\umcfd.exe"); ReleasePath = (Join-Path $RepoRoot "target\release\umcfd.exe") },
  @{ Name = "umcf-cloudrun-control-plane"; Path = (Join-Path $RepoRoot "target\debug\umcf-cloudrun-control-plane.exe"); ReleasePath = (Join-Path $RepoRoot "target\release\umcf-cloudrun-control-plane.exe") }
)

$checksumEntries = @()
$packagedFiles = @()

foreach ($bin in $rustBinaries) {
  $srcPath = $null
  if (Test-Path $bin.ReleasePath) { $srcPath = $bin.ReleasePath }
  elseif (Test-Path $bin.Path) { $srcPath = $bin.Path }

  if ($null -ne $srcPath) {
    $destName = "$($bin.Name)-windows-x86_64.exe"
    $destPath = Join-Path $TargetOut $destName
    Copy-Item -Force $srcPath $destPath
    $hash = (Get-FileHash -Algorithm SHA256 $destPath).Hash.ToLowerInvariant()
    $size = (Get-Item $destPath).Length
    $checksumEntries += "$hash  $destName"
    $packagedFiles += [ordered]@{
      artifact_name = $destName
      platform      = "windows-x86_64"
      sha256        = $hash
      size_bytes    = $size
    }
  }
}

$webDist = Join-Path $RepoRoot "apps\web-dashboard\dist\index.html"
if (Test-Path $webDist) {
  $destName = "umcf-web-dashboard-index.html"
  $destPath = Join-Path $TargetOut $destName
  Copy-Item -Force $webDist $destPath
  $hash = (Get-FileHash -Algorithm SHA256 $destPath).Hash.ToLowerInvariant()
  $size = (Get-Item $destPath).Length
  $checksumEntries += "$hash  $destName"
  $packagedFiles += [ordered]@{
    artifact_name = $destName
    platform      = "web-production"
    sha256        = $hash
    size_bytes    = $size
  }
}

$cfDist = Join-Path $RepoRoot "services\control-plane-cf\dist\worker.mjs"
if (Test-Path $cfDist) {
  $destName = "umcf-control-plane-cf-worker.mjs"
  $destPath = Join-Path $TargetOut $destName
  Copy-Item -Force $cfDist $destPath
  $hash = (Get-FileHash -Algorithm SHA256 $destPath).Hash.ToLowerInvariant()
  $size = (Get-Item $destPath).Length
  $checksumEntries += "$hash  $destName"
  $packagedFiles += [ordered]@{
    artifact_name = $destName
    platform      = "cloudflare-workers"
    sha256        = $hash
    size_bytes    = $size
  }
}

$checksumFile = Join-Path $TargetOut "SHA256SUMS.txt"
$checksumEntries | Set-Content -Encoding UTF8 $checksumFile

$metadata = [ordered]@{
  project          = "Universal Mobile Compute Fabric (UMCF)"
  version          = "0.1.0"
  git_commit       = $gitCommit
  built_at_utc     = $timestamp
  host_os          = [System.Environment]::OSVersion.VersionString
  rustc_version    = ((& rustc --version 2>$null) | Out-String).Trim()
  artifacts        = $packagedFiles
}
$metadata | ConvertTo-Json -Depth 5 | Set-Content -Encoding UTF8 (Join-Path $TargetOut "build-metadata.json")

$sbom = [ordered]@{
  bomFormat    = "CycloneDX"
  specVersion  = "1.5"
  version      = 1
  metadata     = [ordered]@{
    timestamp = $timestamp
    component = [ordered]@{
      type    = "application"
      name    = "universal-mobile-compute-fabric"
      version = "0.1.0"
    }
  }
  components   = @(
    [ordered]@{ type = "library"; name = "umcf-proto"; version = "0.1.0"; licenses = @([ordered]@{ license = @{ id = "Apache-2.0" } }) },
    [ordered]@{ type = "library"; name = "umcf-core"; version = "0.1.0"; licenses = @([ordered]@{ license = @{ id = "Apache-2.0" } }) },
    [ordered]@{ type = "library"; name = "umcf-coordinator"; version = "0.1.0"; licenses = @([ordered]@{ license = @{ id = "Apache-2.0" } }) },
    [ordered]@{ type = "library"; name = "umcf-worker"; version = "0.1.0"; licenses = @([ordered]@{ license = @{ id = "Apache-2.0" } }) },
    [ordered]@{ type = "library"; name = "umcf-scheduler"; version = "0.1.0"; licenses = @([ordered]@{ license = @{ id = "Apache-2.0" } }) },
    [ordered]@{ type = "application"; name = "umcf-cli"; version = "0.1.0"; licenses = @([ordered]@{ license = @{ id = "Apache-2.0" } }) },
    [ordered]@{ type = "application"; name = "umcf-daemon"; version = "0.1.0"; licenses = @([ordered]@{ license = @{ id = "Apache-2.0" } }) },
    [ordered]@{ type = "application"; name = "umcf-control-plane-cloudrun"; version = "0.1.0"; licenses = @([ordered]@{ license = @{ id = "Apache-2.0" } }) }
  )
}
$sbom | ConvertTo-Json -Depth 6 | Set-Content -Encoding UTF8 (Join-Path $TargetOut "sbom.cdx.json")

Write-Host "Packaged $($packagedFiles.Count) artifacts with SHA256SUMS.txt, build-metadata.json, and sbom.cdx.json in $TargetOut"
