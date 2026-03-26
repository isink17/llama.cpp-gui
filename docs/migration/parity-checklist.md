# WinUI to Tauri Parity Checklist

Status key:

- `[ ]` not started
- `[-]` in progress or baseline implemented, but not parity-complete
- `[x]` done

## Core App

- [-] App launches to baseline shell (Windows)
- [ ] App launches to baseline shell (Linux)
- [ ] App launches to baseline shell (macOS)
- [-] Settings are persisted and restored on restart
- [-] Presets are persisted and restored on restart
- [-] Chat history is persisted and restored on restart

## Llama Server Lifecycle

- [-] Start local `llama-server` with configured args
- [-] Stop server process gracefully
- [-] Process crash/failure states are surfaced to UI, with clearer health messages for timeout/unavailable cases
- [-] Health state is visible in UI
- [-] Runtime logs are streamable/viewable in UI

## Downloader

- [-] Download model by URL/source
- [-] Per-download progress updates are emitted
- [-] Active download can be cancelled
- [-] Failure states are surfaced and recoverable

## Chat and Streaming

- [-] Submit chat prompt to backend
- [-] Stream tokens/chunks to UI
- [-] Cancel active generation
- [-] Handle timeout/server unavailable errors in backend and UI status surfaces

## UX Parity

- [-] Settings screen baseline is wired through the typed Tauri API client; WinUI parity still pending
- [-] Chat screen baseline is wired through the typed Tauri API client; WinUI parity still pending
- [-] Downloader screen baseline is wired; WinUI parity still pending
- [-] Presets and history management baselines are wired; WinUI parity still pending

## Packaging and Delivery

- [-] CI validates the migration path on Windows/Linux/macOS
- [-] Artifact naming is stable and documented
- [ ] Tag-trigger release publishing validated

## Notes

- Keep this file updated per migration PR.
- Link issue IDs near completed items when useful.
- Latest implemented scope includes presets/history UI, a typed Tauri API client, `migration-ci`, cross-platform smoke coverage, optional cargo-check smoke validation, and smoke scripts; parity is still in progress.
- Error-handling work now distinguishes timeout, unavailable, and other health failures in the backend and surfaces a clearer refresh failure state in the UI.
- Release workflow now derives a stable tag-based artifact name and shared release paths before upload/publish steps.
