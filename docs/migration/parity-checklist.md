# Tauri Feature Checklist

Status key:

- `[ ]` not started
- `[-]` in progress or baseline implemented
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

## UX

- [-] Settings screen baseline is wired through the typed Tauri API client
- [-] Chat screen baseline is wired through the typed Tauri API client
- [-] Downloader screen baseline is wired
- [-] Presets and history management baselines are wired
- [-] Browser mode is explicit with remote setup/provenance labels
- [-] Remote browser mode can control settings, presets, history, downloads, and chat

## Packaging and Delivery

- [-] CI validates the build path on Windows/Linux/macOS
- [-] Artifact naming is stable and documented
- [ ] Tag-trigger release publishing validated

## Notes

- Keep this file updated per PR.
- Link issue IDs near completed items when useful.
- Latest implemented scope includes presets/history UI, a typed Tauri API client, `migration-ci`, cross-platform smoke coverage, per-OS smoke diagnostics on failure, optional cargo-check smoke validation, and smoke scripts.
- Error-handling work now distinguishes timeout, unavailable, and other health failures in the backend and surfaces a clearer refresh failure state in the UI.
- Release workflow now publishes only the installer formats we want: Windows `.msi`, Linux `.AppImage`, and macOS `.dmg`, with a versioned release title, a simple download section, and generated changelog notes.
