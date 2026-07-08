# 2026-07-08 - Service Schedule Status Text Input Session

## Objective

Close a service-planning validation gap where direct Python callers could pass
non-text requested status values to inserts, list filters, or direct/audited
status updates and receive raw type errors before controlled repository
validation.

## Changes

- Routed service-schedule status normalization through the shared text
  validation helper before allowed-status checks.
- Added regression coverage proving non-text requested statuses are rejected on
  direct inserts, audited inserts, list filters, direct updates, and audited
  updates.
- Verified audited paths do not create project audit events when malformed
  status input is rejected.
- Updated changelog, roadmap, and service-planning documentation.

## Boundaries

- No storage migration, release version bump, tag, Rust API route, or new
  planning workflow was added.
- The guard validates requested status input; persisted corrupted status
  evidence remains covered by the existing read/update guards.

## Validation

- Targeted non-text requested-status regressions:
  `$env:PYTHONPATH='python'; py -m unittest python.tests.test_sqlite_repositories.ProjectRepositoryScheduleTests.test_repository_rejects_non_text_service_schedule_status_on_insert python.tests.test_sqlite_repositories.ProjectRepositoryScheduleTests.test_repository_rejects_non_text_service_schedule_status_on_audit_insert python.tests.test_sqlite_repositories.ProjectRepositoryScheduleTests.test_repository_rejects_non_text_service_schedule_target_status_on_update python.tests.test_sqlite_repositories.ProjectRepositoryScheduleTests.test_repository_rejects_non_text_service_schedule_target_status_on_audit_update python.tests.test_sqlite_repositories.ProjectRepositoryScheduleTests.test_repository_rejects_malformed_service_schedule_list_filters`
  - passed with `5` tests.
- Service-planning repository regression module:
  `$env:PYTHONPATH='python'; py -m unittest python.tests.test_sqlite_repositories.ProjectRepositoryScheduleTests`
  - passed with `68` tests.
- Python package compilation:
  `py -m compileall python/emc_locus`
  - passed.
- Rust test suite:
  `cargo test`
  - passed with `46` agent tests and `167` core tests.
