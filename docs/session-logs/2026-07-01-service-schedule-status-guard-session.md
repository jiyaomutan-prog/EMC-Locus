# 2026-07-01 - Service Schedule Status Guard Session

## Objective

Close the remaining service-planning bypass where direct Python repository
callers could rely on SQLite constraints, instead of repository-owned business
validation, for schedule status values.

## Changes

- Moved the allowed service-schedule status vocabulary into
  `sqlite_repositories.py` next to the repository write logic.
- Added repository-level validation for service-schedule insertion and status
  updates.
- Kept the GUI/CLI action using the same shared status validation.
- Added repository regression tests for invalid status insertion and update
  attempts.
- Updated changelog, roadmap, and service-planning documentation.

## Boundaries

- No storage migration, release/version bump, holiday calendar, capacity
  planning, or Rust service-planning route was added.

## Validation

- Targeted repository schedule tests:
  `$env:PYTHONPATH='python'; py -m unittest python.tests.test_sqlite_repositories.ProjectRepositoryScheduleTests`
  - passed with `4` tests.
- `py -m compileall python\emc_locus` - passed.
- `$env:PYTHONPATH='python'; py -m unittest discover -s python\tests` -
  passed with `87` tests.
- `cargo test` - passed with `44` agent tests and `162` core tests.
