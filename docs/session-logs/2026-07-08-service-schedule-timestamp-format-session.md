# 2026-07-08 - Service Schedule Timestamp Format Session

## Objective

Close a service-planning evidence gap where constraint-bypassed imports could
store non-empty textual but non-canonical `created_at`/`updated_at` values and
still be surfaced through list reads or advanced by direct/audited status
updates.

## Changes

- Added canonical `YYYY-MM-DDTHH:MM:SSZ` UTC timestamp validation for persisted
  service-schedule `created_at` and `updated_at` evidence.
- Reused the persisted-row validation path for list reads, direct status
  updates, and audited status updates.
- Added regression coverage proving non-canonical textual timestamp evidence is
  rejected without mutating planning rows or creating project audit evidence.
- Updated changelog, roadmap, and service-planning documentation.

## Boundaries

- No storage migration, release version bump, tag, Rust API route, or new
  planning workflow was added.
- The guard rejects malformed imported evidence; it does not rewrite existing
  SQLite planning rows.

## Validation

- Targeted non-canonical timestamp regressions:
  `$env:PYTHONPATH='python'; py -m unittest python.tests.test_sqlite_repositories.ProjectRepositoryScheduleTests.test_repository_rejects_non_canonical_service_schedule_timestamp_on_list python.tests.test_sqlite_repositories.ProjectRepositoryScheduleTests.test_repository_rejects_non_canonical_service_schedule_timestamp_on_update python.tests.test_sqlite_repositories.ProjectRepositoryScheduleTests.test_repository_rejects_non_canonical_service_schedule_timestamp_on_audit_update`
  - passed.
- Service-planning repository regression module:
  `$env:PYTHONPATH='python'; py -m unittest python.tests.test_sqlite_repositories.ProjectRepositoryScheduleTests`
  - passed with `58` tests.
- Python package compilation:
  `py -m compileall python/emc_locus`
  - passed.
- Rust test suite:
  `cargo test`
  - passed with `46` agent tests and `167` core tests.
- Whitespace check:
  `git diff --check`
  - passed with only existing CRLF conversion warnings.
