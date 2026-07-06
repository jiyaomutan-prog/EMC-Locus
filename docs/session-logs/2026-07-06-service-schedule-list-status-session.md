# 2026-07-06 - Service Schedule List Status Session

## Objective

Close a service-planning read-side gap where a constraint-bypassed or corrupted
local database import could expose planning rows with non-canonical status
values through repository list reads.

## Changes

- Added repository-side status validation while listing service-schedule rows.
- Added regression coverage that simulates a constraint-bypassed import with an
  unknown persisted status.
- Updated changelog, roadmap, and service-planning documentation.

## Boundaries

- No storage migration, release version bump, tag, Rust API route, or new
  planning workflow was added.
- Current schema CHECK constraints already reject unknown statuses on ordinary
  SQLite writes; this guard protects the Python read path when imported data has
  bypassed those constraints.

## Validation

- Targeted read-side regression:
  `$env:PYTHONPATH='python'; py -m unittest python.tests.test_sqlite_repositories.ProjectRepositoryScheduleTests.test_repository_rejects_unknown_service_schedule_status_on_list`
  - passed with `1` test.
- Service-planning repository regression module:
  `$env:PYTHONPATH='python'; py -m unittest python.tests.test_sqlite_repositories.ProjectRepositoryScheduleTests`
  - passed with `32` tests.
- Python package compilation:
  `py -m compileall python/emc_locus`
  - passed.
- Rust test suite:
  `cargo test`
  - passed with `46` agent tests and `167` core tests.
