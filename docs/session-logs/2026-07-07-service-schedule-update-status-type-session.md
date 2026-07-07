# 2026-07-07 - Service Schedule Update Status Type Session

## Objective

Close a service-planning corruption gap where direct and audited status updates
normalized the persisted current status without first proving it was stored as
text.

## Changes

- Added a repository guard that rejects non-text persisted current statuses
  before direct or audited service-schedule status updates.
- Added regression coverage for constraint-bypassed rows whose current status
  is stored as a SQLite BLOB in both update paths.
- Updated changelog, roadmap, and service-planning documentation.

## Boundaries

- No storage migration, release version bump, tag, Rust API route, or new
  planning workflow was added.
- Padded but known text statuses remain normalized by the existing update path;
  only non-text persisted statuses are newly rejected.

## Validation

- Service-planning repository regression module:
  `$env:PYTHONPATH='python'; py -m unittest python.tests.test_sqlite_repositories.ProjectRepositoryScheduleTests`
  - passed with `41` tests.
- Python package compilation:
  `py -m compileall python/emc_locus`
  - passed.
- Rust test suite:
  `cargo test`
  - passed with `46` agent tests and `167` core tests.
