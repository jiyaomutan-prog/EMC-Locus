# 2026-07-07 - Service Schedule Update Block Validation Session

## Objective

Close a service-planning consistency gap where repository list reads rejected
corrupted planning rows with invalid business-day blocks or malformed planning
context, but direct and audited status updates could still advance the same
rows.

## Changes

- Reused the persisted service-schedule row validation path before direct and
  audited status updates.
- Added regression coverage proving weekend planning rows imported outside the
  repository guards cannot be advanced.
- Verified the audited path leaves no project audit evidence when the corrupted
  row is rejected.
- Updated changelog, roadmap, and service-planning documentation.

## Boundaries

- No storage migration, release version bump, tag, Rust API route, or new
  planning workflow was added.
- The update guard validates imported rows before mutation; it does not rewrite
  corrupted SQLite records.

## Validation

- Service-planning repository regression module:
  `$env:PYTHONPATH='python'; py -m unittest python.tests.test_sqlite_repositories.ProjectRepositoryScheduleTests`
  - passed with `46` tests.
