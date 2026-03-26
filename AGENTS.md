# Migration Agents

This file defines subagent ownership for the Tauri multiplatform migration.

## Global Rules

- Integration branch: `feature/multiplatform_support` (created from `master`).
- Every work item must have a GitHub issue.
- Branch format for each work item: `feature/migration/<issue-number>`.
- Every MR/PR must link its issue in the description (`Closes #<issue-number>`).
- Feature branches merge into `feature/multiplatform_support` first.
- Final consolidation MR/PR is `feature/multiplatform_support -> master`.

## Agent A: Backend Core (Rust/Tauri)

- Scope:
  - Settings, presets, and history persistence.
  - `llama-server` process lifecycle (start/stop/health/log handling).
  - Downloader with progress and cancel support.
  - Chat request flow and streaming orchestration.
- Owns:
  - `src-tauri/src/core/*`
  - `src-tauri/src/services/*`
  - `src-tauri/src/commands/*`
  - `src-tauri/src/state/*`

## Agent B: Frontend App (Tauri UI)

- Scope:
  - App shell, settings screens, chat screen, downloader UI, and presets UI.
  - Frontend state model and Rust command/event wiring.
- Owns:
  - `ui/src/components/*`
  - `ui/src/features/*`
  - `ui/src/state/*`
  - `ui/src/lib/tauri/*`

## Agent C: CI/CD and Packaging

- Scope:
  - GitHub Actions matrix builds for Windows/Linux/macOS.
  - Artifact naming, release asset publishing, and tag-trigger validation.
- Owns:
  - `.github/workflows/*`
  - `src-tauri/tauri.conf.json`
  - `scripts/release/*`

## Agent D: QA, Parity, and Documentation

- Scope:
  - Parity checklist versus WinUI app features.
  - Smoke tests and migration runbook.
  - Developer onboarding docs for local build and release.
- Owns:
  - `docs/migration/*`
  - `README.md`
  - `tests/smoke/*`

## Initial Issue Backlog Template

- `#<n1>` Tauri scaffold + baseline app startup.
- `#<n2>` Rust settings/presets/history service.
- `#<n3>` Rust `llama-server` process manager.
- `#<n4>` Rust downloader service with progress/cancel.
- `#<n5>` Rust chat request/streaming service.
- `#<n6>` UI shell + settings + chat baseline wiring.
- `#<n7>` CI matrix + release artifacts on tag.
- `#<n8>` Parity checklist + migration docs.

Use branch names like `feature/migration/<n1>`, `feature/migration/<n2>`, etc.

## Current Progress Snapshot

- `#<n1>` Tauri scaffold + baseline app startup: in progress on `feature/migration/1`
- `#<n2>` Rust settings/presets/history service: in progress on `feature/migration/1`
- `#<n3>` Rust `llama-server` process manager: in progress on `feature/migration/1`
- `#<n7>` CI matrix + release artifacts on tag: in progress on `feature/migration/1`
- `#<n8>` Parity checklist + migration docs: in progress on `feature/migration/1`

## Module Map

- Legacy WinUI app:
  - `App.xaml`, `Views/*`, `Services/*`, `Models/*`, `Converters/*`
- Tauri backend:
  - `src-tauri/src/core/*`
  - `src-tauri/src/services/*`
  - `src-tauri/src/commands/*`
  - `src-tauri/src/state/*`
- Tauri frontend:
  - `ui/src/*`
- Migration process/docs:
  - `docs/migration/*`
  - `tests/smoke/*`
  - `CONTRIBUTING.md`
  - `README.md`
