# WinUI to Tauri Parity Checklist

Status key:

- `[ ]` not started
- `[-]` in progress
- `[x]` done

## Core App

- [ ] App launches to baseline shell (Windows)
- [ ] App launches to baseline shell (Linux)
- [ ] App launches to baseline shell (macOS)
- [ ] Settings are persisted and restored on restart
- [ ] Presets are persisted and restored on restart
- [ ] Chat history is persisted and restored on restart

## Llama Server Lifecycle

- [ ] Start local `llama-server` with configured args
- [ ] Stop server process gracefully
- [ ] Process crash/failure is surfaced to UI
- [ ] Health state is visible in UI
- [ ] Runtime logs are streamable/viewable in UI

## Downloader

- [ ] Download model by URL/source
- [ ] Per-download progress updates are emitted
- [ ] Active download can be cancelled
- [ ] Failure states are surfaced and recoverable

## Chat and Streaming

- [ ] Submit chat prompt to backend
- [ ] Stream tokens/chunks to UI
- [ ] Cancel active generation
- [ ] Handle timeout/server unavailable errors

## UX Parity

- [ ] Settings screen parity with WinUI scope
- [ ] Chat screen parity with WinUI scope
- [ ] Downloader screen parity with WinUI scope
- [ ] Presets management parity with WinUI scope

## Packaging and Delivery

- [ ] CI builds artifacts on Windows/Linux/macOS
- [ ] Artifact naming is stable and documented
- [ ] Tag-trigger release publishing validated

## Notes

- Keep this file updated per migration PR.
- Link issue IDs near completed items when useful.
