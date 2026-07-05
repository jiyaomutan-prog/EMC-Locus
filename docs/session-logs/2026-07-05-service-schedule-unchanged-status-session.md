# 2026-07-05 - Service Schedule Unchanged Status Session

## Objective

Close a narrow service-planning audit accuracy gap where duplicate status
submissions could record a status-change event even when the planning row status
was already unchanged.

## Changes

- Updated `ProjectRepository.update_service_schedule_status` to reject unchanged
  target statuses before mutating the planning row timestamp.
- Updated `ProjectRepository.update_service_schedule_status_with_audit` to
  reject unchanged target statuses before appending project audit evidence.
- Added repository regression coverage for unchanged direct and audited status
  updates.
- Documented the side-effect-free duplicate submission guard in the changelog,
  roadmap, and service-planning notes.

## Boundaries

- No storage migration, release version bump, tag, Rust API route, or new
  planning workflow was added.
- The guard only rejects status updates whose new status equals the stored
  status; valid transitions such as `planned` to `confirmed` are unchanged.

## Validation

- Targeted repository regressions:
  `$env:PYTHONPATH='python'; py -m unittest python.tests.test_sqlite_repositories.ProjectRepositoryScheduleTests.test_repository_rejects_unchanged_service_schedule_status_update python.tests.test_sqlite_repositories.ProjectRepositoryScheduleTests.test_repository_rejects_unchanged_service_schedule_status_audit python.tests.test_sqlite_repositories.ProjectRepositoryScheduleTests.test_repository_records_service_schedule_status_update_with_project_audit`
  - passed with `3` tests.
- Service-planning repository and action regressions:
  `$env:PYTHONPATH='python'; py -m unittest python.tests.test_sqlite_repositories.ProjectRepositoryScheduleTests python.tests.test_sqlite_repositories.GuiActionTests.test_update_service_schedule_status_records_audit_and_refreshes_bootstrap`
  - passed with `26` tests.
- Python package compilation:
  `py -m compileall python/emc_locus`
  - passed.
- Rust test suite:
  `cargo test`
  - passed with `46` agent tests and `167` core tests.
- Whitespace check:
  `git diff --check`
  - passed.
