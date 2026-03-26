# LlamaCppDesk

LlamaCppDesk is migrating from a Windows-only WinUI app to a Tauri-based cross-platform desktop app.

This repository currently contains both codepaths:

- WinUI app (existing baseline)
- Tauri backend (`src-tauri/`)
- Tauri frontend (`ui/`)

## Project Structure

- WinUI app: `App.xaml`, `Views/`, `Services/`, `Models/`, `Converters/`
- Tauri backend: `src-tauri/`
- Tauri frontend: `ui/`
- Migration docs: `docs/migration/`
- Smoke validation: `tests/smoke/`

## Prerequisites

### Required

- Node.js 22+
- npm
- Rust stable toolchain (with Cargo)

### Platform notes

- Windows:
  - Visual Studio C++ build tools / MSVC runtime must be installed.
  - If you see `LNK1104 msvcrt.lib`, your MSVC toolchain/runtime is incomplete.
- Linux (for Tauri/GTK builds):
  - `pkg-config`
  - `libglib2.0-dev`
  - `libgtk-3-dev`
  - `libwebkit2gtk-4.1-dev`
  - `libayatana-appindicator3-dev`
  - `librsvg2-dev`

## Quick Start (Tauri App)

From repository root:

```powershell
npm --prefix ui install
```

### Run frontend only

```powershell
npm --prefix ui run dev
```

### Build frontend

```powershell
npm --prefix ui run build
```

### Check backend formatting/build

```powershell
cargo fmt --manifest-path src-tauri/Cargo.toml
cargo check --manifest-path src-tauri/Cargo.toml
```

### Run Tauri backend binary directly

```powershell
cargo run --manifest-path src-tauri/Cargo.toml
```

## Smoke Validation

Use smoke scripts before opening/merging migration PRs:

```powershell
.\tests\smoke\Invoke-SmokeChecks.ps1
```

```bash
./tests/smoke/Invoke-SmokeChecks.sh
```

Stricter path (includes `cargo check --locked`):

```powershell
.\tests\smoke\Invoke-SmokeChecks.ps1 -SkipPrereqCheck -IncludeCargoCheck
```

```bash
./tests/smoke/Invoke-SmokeChecks.sh --skip-prereq-check --include-cargo-check
```

Manual checklist: `tests/smoke/smoke-checklist.md`

## CI / Release Notes

- Migration CI: `.github/workflows/migration-ci.yml`
  - Cross-platform validation matrix
  - Smoke checks and per-OS diagnostics artifacts on failure
- Tag build workflow: `.github/workflows/build-on-tag.yml`
  - Stable artifact naming
  - Smoke validation path with cargo-check enabled

## Contribution and Workflow

- Contribution rules: `CONTRIBUTING.md`
- Agent ownership: `AGENTS.md`
- Migration runbook: `docs/migration/runbook.md`
- Parity checklist: `docs/migration/parity-checklist.md`

Branching model:

- Integration branch: `feature/multiplatform_support`
- Work branches: `feature/migration/<issue-number>`
- Migration PRs target: `feature/multiplatform_support`
- PR descriptions must include: `Closes #<issue-number>`

## Current Status

Migration branch currently has baseline implementations for:

- Settings, presets, history persistence
- `llama-server` lifecycle and health/log surfacing
- Downloader start/cancel/status flow
- Chat stream start/cancel/status flow
- Typed Tauri API integration in the UI

Parity hardening and final release consolidation are still in progress.
