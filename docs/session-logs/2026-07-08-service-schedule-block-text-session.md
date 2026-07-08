# 2026-07-08 - Service Schedule Block Text Session

## Objective

Close a service-planning validation gap where direct Python callers or
constraint-bypassed imports could provide non-text `planned_start_at` or
`planned_end_at` values and rely on lower-level parser/type errors instead of
controlled repository validation.

## Changes

- Added explicit text validation for service-schedule planned start/end fields
  before canonical local date-time parsing and business-day block validation.
- Reused the validation for repository inserts, list reads, direct status
  updates, and audited status updates.
- Added regression coverage proving non-text block evidence is rejected without
  persisting rows, mutating status, or creating status-update audit evidence.
- Updated changelog, roadmap, and service-planning documentation.

## Boundaries

- No storage migration, release version bump, tag, Rust API route, holiday
  calendar, or capacity-planning model was added.
- The guard rejects malformed imported evidence; it does not rewrite existing
  SQLite planning rows.

## Validation

- Targeted non-text block regressions:
  `$env:PYTHONPATH='python'; py -m unittest python.tests.test_sqlite_repositories.ProjectRepositoryScheduleTests.test_repository_rejects_non_text_service_schedule_block_on_insert python.tests.test_sqlite_repositories.ProjectRepositoryScheduleTests.test_repository_rejects_non_text_service_schedule_block_on_list python.tests.test_sqlite_repositories.ProjectRepositoryScheduleTests.test_repository_rejects_non_text_service_schedule_block_on_update python.tests.test_sqlite_repositories.ProjectRepositoryScheduleTests.test_repository_rejects_non_text_service_schedule_block_on_audit_update`
  - passed.
- Service-planning repository regression module:
  `$env:PYTHONPATH='python'; py -m unittest python.tests.test_sqlite_repositories.ProjectRepositoryScheduleTests`
  - passed with `62` tests.
- Python package compilation:
  `py -m compileall python/emc_locus`
  - passed.
- Rust test suite:
  `cargo test`
  - passed with `46` agent tests and `167` core tests.
- Whitespace check:
  `git diff --check`
  - passed with only existing CRLF conversion warnings.
