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

## Current Wiring Snapshot

As of this branch, the Tauri backend registers these command groups in `src-tauri/src/lib.rs`:

- persistence/data: settings, presets, and history commands
- process: `llama-server` start/stop/status/logs/health commands
- downloader: start/cancel/status commands
- chat: start/cancel/status commands

### Command / Event Catalog

Backend commands currently registered with Tauri:

- Persistence/data: `get_settings`, `save_settings`, `get_presets`, `save_preset`, `delete_preset`, `get_history`, `append_history`, `clear_history`
- Process: `start_llama_server`, `stop_llama_server`, `get_llama_server_status`, `get_llama_server_logs`, `clear_llama_server_logs`, `check_llama_server_health`
- Downloader: `start_download`, `cancel_download`, `get_download_status`, `get_download_statuses`
- Chat: `start_chat_stream`, `cancel_chat_stream`, `get_chat_stream_status`, `get_chat_stream_statuses`
- Misc: `ping`

Emitted chat event:

- `chat_stream_event`

Frontend wiring is still a baseline bridge in `ui/src/lib/tauri/index.ts` and `ui/src/App.tsx`:

- `invokeCommand(...)` is used for command calls
- `listenToEvent('chat_stream_event', ...)` handles chat stream chunks and errors
- the app currently refreshes status data by polling the registered status commands

## Documents

- `parity-checklist.md`: feature parity tracking between WinUI and Tauri targets
- `runbook.md`: end-to-end execution workflow for migration work items
