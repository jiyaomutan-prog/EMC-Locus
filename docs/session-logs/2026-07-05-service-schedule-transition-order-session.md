# 2026-07-05 - Service Schedule Transition Order Session

## Objective

Close a narrow service-planning workflow gap where repository status updates
could move planning rows backward or skip required intermediate states.

## Changes

- Added an explicit service-schedule status transition map:
  `planned -> confirmed -> in_progress -> completed`, with `cancelled` allowed
  from non-terminal states.
- Applied the rule through the existing direct and audited repository status
  update paths.
- Added repository regression coverage proving backward direct and audited
  status updates leave the row unchanged and audited calls do not append project
  audit evidence.
- Updated changelog, roadmap, and service-planning documentation.

## Boundaries

- No storage migration, release version bump, tag, Rust API route, or new
  operator workflow was added.
- Existing valid confirmation, start, completion, and cancellation transitions
  remain available.

## Validation

- Targeted transition regressions:
  `$env:PYTHONPATH='python'; py -m unittest python.tests.test_sqlite_repositories.ProjectRepositoryScheduleTests.test_repository_rejects_backward_service_schedule_status_update python.tests.test_sqlite_repositories.ProjectRepositoryScheduleTests.test_repository_rejects_backward_service_schedule_status_audit python.tests.test_sqlite_repositories.ProjectRepositoryScheduleTests.test_repository_rejects_terminal_service_schedule_status_update python.tests.test_sqlite_repositories.ProjectRepositoryScheduleTests.test_repository_rejects_terminal_service_schedule_status_audit python.tests.test_sqlite_repositories.ProjectRepositoryScheduleTests.test_repository_records_service_schedule_status_update_with_project_audit`
  - passed with `5` tests.
- Service-schedule repository regressions:
  `$env:PYTHONPATH='python'; py -m unittest python.tests.test_sqlite_repositories.ProjectRepositoryScheduleTests`
  - passed with `29` tests.
- Python package compilation:
  `py -m compileall python/emc_locus`
  - passed.
- Rust test suite:
  `cargo test`
  - passed with `46` agent tests and `167` core tests.
