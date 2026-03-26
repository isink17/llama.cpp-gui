#!/usr/bin/env pwsh
[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$commands = @('cargo', 'npm')
$missing = @()

foreach ($command in $commands) {
  if (-not (Get-Command $command -ErrorAction SilentlyContinue)) {
    $missing += $command
  }
}

if ($missing.Count -gt 0) {
  Write-Host ('Missing required commands: {0}' -f ($missing -join ', ')) -ForegroundColor Red
  exit 1
}

Write-Host 'Smoke prereqs available: cargo, npm' -ForegroundColor Green
