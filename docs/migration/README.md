# Migration Documentation

This folder tracks migration progress from the current WinUI app to the Tauri multiplatform app.

## Branching Rules

- Integration branch: `feature/multiplatform_support`
- Work item branch: `feature/migration/<issue-number>`
- Migration PRs target `feature/multiplatform_support`
- PR description must include `Closes #<issue-number>`
- Final consolidation PR: `feature/multiplatform_support -> master`

## Backlog Mapping

- `#<n1>` Tauri scaffold + baseline app startup
- `#<n2>` Rust settings/presets/history service
- `#<n3>` Rust `llama-server` process manager
- `#<n4>` Rust downloader service with progress/cancel
- `#<n5>` Rust chat request/streaming service
- `#<n6>` UI shell + settings + chat baseline wiring
- `#<n7>` CI matrix + release artifacts on tag
- `#<n8>` Parity checklist + migration docs

## Documents

- `parity-checklist.md`: feature parity tracking between WinUI and Tauri targets
- `runbook.md`: end-to-end execution workflow for migration work items
