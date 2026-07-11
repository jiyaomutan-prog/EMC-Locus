# 2026-07-11 - Service Schedule Required Text Input Session

## Objective

Close a service-planning validation gap where direct repository callers could
pass non-text required planning fields, status-update item codes, or audited
actors and receive a raw helper attribute error before controlled repository
validation.

## Changes

- Hardened the shared required-text helper so non-text required values raise a
  controlled field-specific `ValueError` instead of calling `.strip()` on the
  malformed object.
- Added regression coverage proving direct service-schedule inserts reject
  non-text required planning fields before creating rows.
- Added regression coverage proving direct service-schedule status updates
  reject a non-text item code before lookup or mutation.
- Added regression coverage proving audited service-schedule inserts reject a
  non-text actor before creating either a planning row or a project audit event.
- Added regression coverage proving audited service-schedule status updates
  reject a non-text actor before mutating the planning row or creating audit
  evidence.
- Updated changelog, roadmap, and service-planning documentation.

## Boundaries

- No storage migration, release version bump, tag, Rust API route, or new
  planning workflow was added.
- The change only normalizes validation failure shape for required text input;
  it does not rewrite existing repository data.

## Validation

- Targeted required-text regressions:
  `$env:PYTHONPATH='python'; py -m unittest python.tests.test_sqlite_repositories.ProjectRepositoryScheduleTests.test_repository_rejects_non_text_service_schedule_required_text_on_insert python.tests.test_sqlite_repositories.ProjectRepositoryScheduleTests.test_repository_rejects_non_text_service_schedule_required_text_on_audit_insert python.tests.test_sqlite_repositories.ProjectRepositoryScheduleTests.test_repository_rejects_non_text_service_schedule_item_code_on_update python.tests.test_sqlite_repositories.ProjectRepositoryScheduleTests.test_repository_rejects_non_text_service_schedule_actor_on_audit_insert python.tests.test_sqlite_repositories.ProjectRepositoryScheduleTests.test_repository_rejects_non_text_service_schedule_actor_on_audit_update`
  - passed with `5` tests.
- Service-planning repository regression module:
  `$env:PYTHONPATH='python'; py -m unittest python.tests.test_sqlite_repositories.ProjectRepositoryScheduleTests`
  - passed with `73` tests.
- Python package compilation:
  `py -m compileall python/emc_locus`
  - passed.
- Rust test suite:
  `cargo test`
  - passed with `46` agent tests and `167` core tests.
