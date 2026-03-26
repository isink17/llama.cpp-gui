# Contributing

## Multiplatform Migration Workflow

- Long-lived integration branch: `feature/multiplatform_support` (created from `master`).
- One branch per issue: `feature/migration/<issue-number>`.
- Every PR for migration work must:
  - target `feature/multiplatform_support`
  - include `Closes #<issue-number>` in the PR description
  - stay within the issue scope
- Only final consolidation PR targets `master`:
  - source: `feature/multiplatform_support`
  - target: `master`

## Recommended Flow

1. Create/assign issue for a scoped migration part.
2. Create branch `feature/migration/<issue-number>`.
3. Implement only issue scope.
4. Open PR to `feature/multiplatform_support` and link issue.
5. Rebase/merge with latest integration branch before final review.

## Migration Docs

- Process runbook: `docs/migration/runbook.md`
- Parity tracker: `docs/migration/parity-checklist.md`
- Smoke baseline: `tests/smoke/smoke-checklist.md`
