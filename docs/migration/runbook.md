# Migration Runbook

This runbook defines the standard execution flow for each migration work item.

## 1. Create/Refine Issue

1. Open a GitHub issue using the `Migration Part` template.
2. Confirm scope, dependencies, acceptance criteria, and validation plan.
3. Assign one owner based on agent area:
   - Backend Core (Rust/Tauri)
   - Frontend App (Tauri UI)
   - CI/CD and Packaging
   - QA, Parity, and Documentation

## 2. Branch and Implement

1. Create branch from `feature/multiplatform_support`:
   - `feature/migration/<issue-number>`
2. Limit changes to issue scope.
3. Keep ownership boundaries from `AGENTS.md`.

## 3. Validate Locally

1. Run relevant build/test commands for touched areas.
2. Record manual verification steps in issue/PR notes.
3. If parity behavior changes, update `parity-checklist.md`.
4. Use `[-]` for baseline-implemented items that were exercised locally; keep `[ ]` for gaps that are still unverified.
5. Reserve `[x]` for end-to-end validation, not just code presence.
6. Use the smoke scripts in `tests/smoke/` for the fast migration smoke pass:
   - `Invoke-SmokeChecks.ps1`
   - `Invoke-SmokeChecks.sh`
7. The `migration-ci` workflow runs `tests/smoke/Invoke-SmokeChecks.sh --skip-prereq-check` before `cargo check`.

## 4. Open PR to Integration Branch

1. Open PR with base `feature/multiplatform_support`.
2. Include `Closes #<issue-number>` in PR description.
3. Complete the checklist from `.github/pull_request_template.md`.

## Current Wiring Snapshot

As of this branch, the Tauri backend already registers:

- persistence/data commands for settings, presets, and history
- process commands for `llama-server` start/stop/status/logs/health
- downloader commands for start/cancel/status
- chat commands for start/cancel/status

The frontend baseline is wired through `ui/src/lib/tauri/index.ts` and `ui/src/App.tsx`:

- command calls go through `invokeCommand(...)`
- chat streaming listens to `chat_stream_event`
- status panels are refreshed by polling the registered status commands

## 5. Merge and Track

1. Merge PR after review and passing CI.
2. Update issue status and linked follow-up issues.
3. Keep this runbook and checklist current as process evolves.

## Final Release Consolidation

After migration backlog completion:

1. Open one PR from `feature/multiplatform_support` to `master`.
2. Validate release workflow and artifacts.
3. Include final parity checklist sign-off.
