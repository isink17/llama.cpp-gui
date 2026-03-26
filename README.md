# LlamaCppDesk

LlamaCppDesk is currently transitioning from a Windows-only WinUI desktop app to a Tauri-based multiplatform app.

## Repository Layout

- WinUI app (current production baseline):
  - `App.xaml`, `Views/*`, `Services/*`, `Models/*`, `Converters/*`
- Tauri backend (migration target):
  - `src-tauri/*`
- Tauri frontend (migration target):
  - `ui/*`
- Migration docs and QA:
  - `docs/migration/*`
  - `tests/smoke/*`

## Branching Model (Migration)

- Integration branch: `feature/multiplatform_support`
- Work-item branches: `feature/migration/<issue-number>`
- Migration PRs target: `feature/multiplatform_support`
- Final consolidation PR target: `master`

Every migration PR should include:

- `Closes #<issue-number>`

## Current Migration Scope

- `#<n1>` Tauri scaffold + baseline app startup
- `#<n2>` Rust settings/presets/history service
- `#<n3>` Rust `llama-server` process manager
- `#<n4>` Rust downloader service with progress/cancel
- `#<n5>` Rust chat request/streaming service
- `#<n6>` UI shell + settings + chat baseline wiring
- `#<n7>` CI matrix + release artifacts on tag
- `#<n8>` Parity checklist + migration docs

## Local Development

### WinUI app

Use your standard .NET/Windows App SDK workflow for the existing WinUI codebase.

### Tauri migration app

Frontend:

```powershell
npm --prefix ui install
npm --prefix ui run build
```

Backend:

```powershell
cargo fmt --manifest-path src-tauri/Cargo.toml
cargo check --manifest-path src-tauri/Cargo.toml
```

Note: if `cargo check` fails with `LNK1104 msvcrt.lib`, your local MSVC toolchain/runtime setup is incomplete.

## Process and Docs

- Contribution rules: `CONTRIBUTING.md`
- Agent ownership and scope: `AGENTS.md`
- Migration runbook: `docs/migration/runbook.md`
- Parity checklist: `docs/migration/parity-checklist.md`
- Smoke checklist: `tests/smoke/smoke-checklist.md`
