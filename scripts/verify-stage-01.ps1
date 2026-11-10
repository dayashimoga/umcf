<#
.SYNOPSIS
  scripts/verify-stage-01.ps1 — UMCF Stage 1 Master Verification & Certification.
  Subphase S1.P3.SP3
#>
[CmdletBinding()]
param()

$ErrorActionPreference = "Stop"

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$RepoRoot = (Resolve-Path (Join-Path $ScriptDir "..")).Path
Set-Location $RepoRoot

& python (Join-Path $ScriptDir "verify_stage_01.py")
if ($LASTEXITCODE -ne 0) { throw "Stage 1 verification failed" }
