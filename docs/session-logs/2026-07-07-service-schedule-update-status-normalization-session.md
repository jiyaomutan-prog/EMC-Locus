# 2026-07-07 - Service Schedule Update Status Normalization Session

## Objective

Close a service-planning consistency gap where direct and audited status
updates read the persisted current planning status without normalizing it, even
though inserts, filters, and list reads already normalize known status text.

## Changes

- Normalized the current persisted service-schedule status before transition
  validation in direct repository status updates.
- Applied the same normalization before audited status updates so audit payloads
  record canonical previous/new status evidence.
- Added regression coverage for constraint-bypassed rows with padded but known
  current statuses in both direct and audited update paths.
- Updated changelog, roadmap, and service-planning documentation.

## Boundaries

- No storage migration, release version bump, tag, Rust API route, or new
  planning workflow was added.
- Unknown persisted statuses remain rejected; only known statuses with
  surrounding whitespace are normalized before update handling.

## Validation

- Targeted update regressions:
  `$env:PYTHONPATH='python'; py -m unittest python.tests.test_sqlite_repositories.ProjectRepositoryScheduleTests.test_repository_normalizes_existing_service_schedule_status_on_update python.tests.test_sqlite_repositories.ProjectRepositoryScheduleTests.test_repository_normalizes_existing_service_schedule_status_on_audit_update`
  - passed with `2` tests.
- Service-planning repository regression module:
  `$env:PYTHONPATH='python'; py -m unittest python.tests.test_sqlite_repositories.ProjectRepositoryScheduleTests`
  - passed with `39` tests.
- Python package compilation:
  `py -m compileall python/emc_locus`
  - passed.
- Rust test suite:
  `cargo test`
  - passed with `46` agent tests and `167` core tests.
