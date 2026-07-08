# 2026-07-08 - Service Schedule Timestamp Evidence Session

## Objective

Close a service-planning read/update integrity gap where constraint-bypassed
imports could store blank or non-text `created_at`/`updated_at` evidence and
still be surfaced through list reads or advanced by status updates.

## Changes

- Revalidated persisted service-schedule `created_at` and `updated_at` values
  while listing rows.
- Reused the same persisted-row validation before direct and audited status
  updates mutate planning rows.
- Added regression coverage for corrupted timestamp evidence on list and update
  paths.
- Updated changelog, roadmap, and service-planning documentation.

## Boundaries

- No storage migration, release version bump, tag, Rust API route, or new
  planning workflow was added.
- The guard rejects corrupted imported evidence; it does not rewrite existing
  SQLite planning rows.

## Validation

- Targeted timestamp regressions:
  `$env:PYTHONPATH='python'; py -m unittest python.tests.test_sqlite_repositories.ProjectRepositoryScheduleTests.test_repository_rejects_invalid_service_schedule_timestamps_on_list python.tests.test_sqlite_repositories.ProjectRepositoryScheduleTests.test_repository_rejects_invalid_service_schedule_timestamps_on_update`
  - passed.
- Service-planning repository regression module:
  `$env:PYTHONPATH='python'; py -m unittest python.tests.test_sqlite_repositories.ProjectRepositoryScheduleTests`
  - passed with `53` tests.
- Python package compilation:
  `py -m compileall python/emc_locus`
  - passed.
- Rust test suite:
  `cargo test`
  - passed with `46` agent tests and `167` core tests.
- Whitespace check:
  `git diff --check`
  - passed with only existing CRLF conversion warnings.
