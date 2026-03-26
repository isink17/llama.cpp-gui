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
- [ ] Error states are visible

## Chat

- [ ] Prompt request succeeds
- [ ] Streaming response is visible
- [ ] Generation cancel works

## Downloader

- [ ] Download starts
- [ ] Progress updates are visible
- [ ] Cancel works

## CI/Artifacts (when touched)

- [ ] Relevant CI workflow passes
- [ ] Artifact output path/name validated
