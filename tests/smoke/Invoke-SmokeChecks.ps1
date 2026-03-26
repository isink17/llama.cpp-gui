#!/usr/bin/env pwsh
[CmdletBinding()]
param(
  [switch]$SkipPrereqCheck,
  [switch]$IncludeCargoCheck,
  [Alias('h')]
  [switch]$Help
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$uiDir = Join-Path $repoRoot 'ui'
$tauriDir = Join-Path $repoRoot 'src-tauri'
$cargoToml = Join-Path $tauriDir 'Cargo.toml'

function Test-SmokePrereqs {
  $commands = @('cargo', 'npm')
  $missing = @()

  foreach ($command in $commands) {
    if (-not (Get-Command $command -ErrorAction SilentlyContinue)) {
      $missing += $command
    }
  }

  if ($missing.Count -gt 0) {
    throw ('Missing required commands: {0}' -f ($missing -join ', '))
  }

  Write-Host 'Smoke prereqs available: cargo, npm' -ForegroundColor Green
}

function Invoke-CheckedCommand {
  param(
    [Parameter(Mandatory = $true)]
    [string]$FilePath,

    [Parameter(Mandatory = $true)]
    [string[]]$ArgumentList,

    [Parameter(Mandatory = $true)]
    [string]$WorkingDirectory
  )

  Write-Host ('Running: {0} {1}' -f $FilePath, ($ArgumentList -join ' ')) -ForegroundColor Cyan
  Push-Location $WorkingDirectory
  try {
    & $FilePath @ArgumentList
    if ($LASTEXITCODE -ne 0) {
      throw ('Command failed with exit code {0}: {1} {2}' -f $LASTEXITCODE, $FilePath, ($ArgumentList -join ' '))
    }
  }
  finally {
    Pop-Location
  }
}

function Show-Help {
  $helpText = @'
Usage: Invoke-SmokeChecks.ps1 [-SkipPrereqCheck] [-IncludeCargoCheck] [-Help]

Runs the fast smoke checks for the migration slice.

Optional flags:
  -IncludeCargoCheck  Run `cargo check --locked` after the default smoke pass.
'@

  Write-Host $helpText
}

if ($Help) {
  Show-Help
  exit 0
}

if (-not $SkipPrereqCheck) {
  Test-SmokePrereqs
}

if (-not (Test-Path $uiDir)) {
  throw "Missing UI directory: $uiDir"
}

if (-not (Test-Path $cargoToml)) {
  throw "Missing Cargo manifest: $cargoToml"
}

Invoke-CheckedCommand -FilePath 'npm' -ArgumentList @('run', 'build') -WorkingDirectory $uiDir
Invoke-CheckedCommand -FilePath 'cargo' -ArgumentList @('fmt', '--check', '--manifest-path', $cargoToml) -WorkingDirectory $tauriDir

if ($IncludeCargoCheck) {
  Invoke-CheckedCommand -FilePath 'cargo' -ArgumentList @('check', '--locked') -WorkingDirectory $tauriDir
}

Write-Host 'Smoke checks completed successfully.' -ForegroundColor Green
