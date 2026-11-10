<#
.SYNOPSIS
  scripts/generate-sbom-and-manifest.ps1 — UMCF SBOM & Release Manifest Generator.
  Subphase S1.P3.SP3
#>
[CmdletBinding()]
param()

$ErrorActionPreference = "Stop"

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$RepoRoot = (Resolve-Path (Join-Path $ScriptDir "..")).Path
Set-Location $RepoRoot

& python (Join-Path $ScriptDir "generate_sbom_and_manifest.py")
if ($LASTEXITCODE -ne 0) { throw "SBOM & manifest generation failed" }
