# Smoke Test Baseline

This folder contains lightweight smoke validation for migration increments.

## Goal

Provide fast confidence checks that the current migration slice is still runnable and does not regress critical flows.

## Scripts

- `Invoke-SmokeChecks.ps1` runs the fast automated smoke pass.
- `Test-SmokePrereqs.ps1` checks that required local commands exist before the smoke pass runs.

## Usage

Run from the repository root with PowerShell:

```powershell
.\tests\smoke\Test-SmokePrereqs.ps1
.\tests\smoke\Invoke-SmokeChecks.ps1
```

Optional: skip the prereq check when you already know the environment is ready.

```powershell
.\tests\smoke\Invoke-SmokeChecks.ps1 -SkipPrereqCheck
```

## What The Smoke Pass Covers

- `npm run build` in `ui/`
- `cargo fmt --check --manifest-path src-tauri/Cargo.toml`
- Basic command existence checks for `cargo` and `npm`

## Notes

- The scripts are non-destructive.
- They are intended to stay fast enough for local pre-PR validation.

## Manual Checklist

- Use `smoke-checklist.md` as a manual validation gate per migration PR.

## Scope

- Startup checks
- Settings/persistence checks
- Chat request/streaming checks
- Downloader checks
- Basic packaging checks
