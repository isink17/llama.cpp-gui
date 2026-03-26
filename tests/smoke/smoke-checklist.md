# Migration Smoke Checklist

Run this checklist for each migration PR when relevant.

## Startup

- [ ] App starts without crash
- [ ] Main shell renders

## Persistence

- [ ] Settings save and reload
- [ ] Presets save and reload
- [ ] Chat history save and reload

## Llama Process

- [ ] `llama-server` starts
- [ ] `llama-server` stops cleanly
- [ ] Health check failure states are visible
- [ ] Refresh failure surfacing is visible in the UI

## Chat

- [ ] Prompt request succeeds
- [ ] Streaming response is visible
- [ ] Generation cancel works
- [ ] Timeout errors are surfaced clearly
- [ ] Server unavailable errors are surfaced clearly

## Downloader

- [ ] Download starts
- [ ] Progress updates are visible
- [ ] Cancel works

## CI/Artifacts (when touched)

- [ ] Relevant CI workflow passes
- [ ] Optional cargo-check smoke path executed when backend/runtime code changes
- [ ] Artifact output path/name validated
