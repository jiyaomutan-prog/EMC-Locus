# 2026-07-05 - Service Schedule Terminal Status Session

## Objective

Close a narrow service-planning traceability gap where completed or cancelled
planning rows could still be reopened through repository status updates.

## Changes

- Added a terminal service-schedule status transition guard for `completed` and
  `cancelled` rows.
- Applied the guard to both direct and audited repository status update paths.
- Added repository regression coverage proving terminal rows keep their closed
  status and audited calls do not append a second status-change event.
- Documented the terminal status rule in the changelog, roadmap, and
  service-planning notes.

## Boundaries

- No storage migration, release version bump, tag, Rust API route, or new
  operator workflow was added.
- The change only prevents updates after a row has already reached a terminal
  status; normal planned, confirmed, and in-progress updates remain unchanged.

## Validation

- Targeted terminal status regressions:
  `$env:PYTHONPATH='python'; py -m unittest python.tests.test_sqlite_repositories.ProjectRepositoryScheduleTests.test_repository_rejects_terminal_service_schedule_status_update python.tests.test_sqlite_repositories.ProjectRepositoryScheduleTests.test_repository_rejects_terminal_service_schedule_status_audit`
  - passed with `2` tests.
- Targeted status audit compatibility regression:
  `$env:PYTHONPATH='python'; py -m unittest python.tests.test_sqlite_repositories.ProjectRepositoryScheduleTests.test_repository_rejects_terminal_service_schedule_status_update python.tests.test_sqlite_repositories.ProjectRepositoryScheduleTests.test_repository_rejects_terminal_service_schedule_status_audit python.tests.test_sqlite_repositories.ProjectRepositoryScheduleTests.test_repository_records_service_schedule_status_update_with_project_audit`
  - passed with `3` tests.
- Python package compilation:
  `py -m compileall python/emc_locus`
  - passed.
- Rust test suite:
  `cargo test`
  - passed with `46` agent tests and `167` core tests.
- Whitespace check:
  `git diff --check`
  - passed.
