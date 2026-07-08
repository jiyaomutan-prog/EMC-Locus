# 2026-07-08 - Service Schedule Insert Item Code Normalization Session

## Objective

Close a service-planning consistency gap where reads and status updates treated
persisted planning item codes by their normalized value, but repository inserts
only checked exact stored item codes for duplicates.

## Changes

- Matched service-schedule insert duplicate checks against trimmed persisted
  planning item codes.
- Added regression coverage proving a constraint-bypassed padded planning code
  blocks a later canonical insert with the same normalized item code.
- Updated changelog, roadmap, and service-planning documentation.

## Boundaries

- No storage migration, release version bump, tag, Rust API route, or new
  planning workflow was added.
- The guard prevents new normalized duplicates; it does not rewrite imported
  SQLite `item_code` values.

## Validation

- Targeted normalized duplicate regression:
  `$env:PYTHONPATH='python'; py -m unittest python.tests.test_sqlite_repositories.ProjectRepositoryScheduleTests.test_repository_rejects_normalized_duplicate_service_schedule_item_code_on_insert`
  - passed.
- Service-planning repository regression module:
  `$env:PYTHONPATH='python'; py -m unittest python.tests.test_sqlite_repositories.ProjectRepositoryScheduleTests`
  - passed with `50` tests.
