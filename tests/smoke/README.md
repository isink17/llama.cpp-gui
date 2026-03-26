# Smoke Test Baseline

This folder contains lightweight smoke validation for migration increments.

## Goal

Provide fast confidence checks that the current migration slice is still runnable and does not regress critical flows.

## Scripts

- `Invoke-SmokeChecks.ps1` runs the fast automated smoke pass.
- `Invoke-SmokeChecks.sh` runs the same smoke pass from bash on macOS/Linux or any bash-capable shell.
- `Test-SmokePrereqs.ps1` checks that required local commands exist before the smoke pass runs.

## Usage

Run from the repository root with PowerShell:

```powershell
.\tests\smoke\Test-SmokePrereqs.ps1
.\tests\smoke\Invoke-SmokeChecks.ps1
.\tests\smoke\Invoke-SmokeChecks.ps1 -Help
```

Run from the repository root with bash:

```bash
./tests/smoke/Invoke-SmokeChecks.sh
```

Optional: skip the prereq check when you already know the environment is ready.

```powershell
.\tests\smoke\Invoke-SmokeChecks.ps1 -SkipPrereqCheck
.\tests\smoke\Invoke-SmokeChecks.ps1 -SkipPrereqCheck -IncludeCargoCheck
```

```bash
./tests/smoke/Invoke-SmokeChecks.sh --skip-prereq-check
./tests/smoke/Invoke-SmokeChecks.sh --skip-prereq-check --include-cargo-check
```

The optional `IncludeCargoCheck` / `--include-cargo-check` flag adds `cargo check --locked`
after the default smoke pass. Leave it off for the fastest validation path.

Use `-Help` in PowerShell or `--help` in bash to print the script usage summary.

## Execution Order

The smoke pass is intentionally ordered to catch the cheapest regressions first:

1. `npm run build` in `ui/`
2. `cargo fmt --check --manifest-path src-tauri/Cargo.toml`
3. `cargo check --locked` when the optional cargo-check flag is enabled

Use `--skip-prereq-check` only when `cargo` and `npm` are already confirmed available, or inside CI after setup.

## What The Smoke Pass Covers

- `npm run build` in `ui/`
- `cargo fmt --check --manifest-path src-tauri/Cargo.toml`
- Basic command existence checks for `cargo` and `npm`
- Fast signal for frontend refresh failure surfacing and process health visibility
- Manual follow-up for chat timeout and unavailable-state handling

## Notes

- The scripts are non-destructive.
- They are intended to stay fast enough for local pre-PR validation.
- Use the manual checklist for behavioral coverage that cannot be fully asserted by the smoke scripts alone.

## Manual Checklist

- Use `smoke-checklist.md` as a manual validation gate per migration PR.

## Scope

- Startup checks
- Settings/persistence checks
- Chat request/streaming checks
- Downloader checks
- Basic packaging checks
