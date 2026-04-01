# LlamaCppDesk

LlamaCppDesk is a Tauri-based cross-platform desktop app for managing and chatting with local llama.cpp models.

If you just want to use the app, download the installer from the GitHub Release page that matches your computer:

- Windows: `.msi`
- Linux: `.AppImage`
- macOS: `.dmg`

You do not need the source code unless you want to build or change the app yourself.

## Project Structure

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

This starts the browser-only Vite app on the default port (`5173`). Tauri features are disabled in that mode.

For remote browser mode against a Tauri backend, see [docs/migration/remote-mode.md](docs/migration/remote-mode.md).

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

`cargo run` will launch the Tauri shell and start the UI through the same `npm --prefix ui run dev` command used for browser-only development.

## Data Directory

The app stores user data (settings, presets, history, downloaded models) in Tauri's `app_data_dir()` under a `migration-data/` subdirectory.

Remote browser mode stores its connection settings in browser local storage and uses `LLAMACPPDESK_REMOTE_BIND` plus `LLAMACPPDESK_REMOTE_TOKEN` on the backend host.

| OS      | Path                                                             |
|---------|------------------------------------------------------------------|
| Windows | `%APPDATA%\com.llamacppdesk\migration-data\`                     |
| Linux   | `~/.local/share/com.llamacppdesk/migration-data/`                |
| macOS   | `~/Library/Application Support/com.llamacppdesk/migration-data/` |

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
  - Builds Tauri app via `cargo tauri build`
  - Uploads the platform installers only: Windows `.msi`, Linux `.AppImage`, macOS `.dmg`
  - Creates a GitHub release named `LlamaCppDesk vX.Y.Z` with a short download section and a changelog, then attaches only those installers

## Contribution and Workflow

- Contribution rules: `CONTRIBUTING.md`
- Agent ownership: `AGENTS.md`
- Migration runbook: `docs/migration/runbook.md`
- Parity checklist: `docs/migration/parity-checklist.md`

## Current Status

The Tauri app includes:

- Settings, presets, history persistence
- `llama-server` lifecycle and health/log surfacing
- Downloader start/cancel/status flow
- Chat stream start/cancel/status flow
- Typed Tauri API integration in the UI
