# 2026-07-07 - Service Schedule List Status Normalization Session

## Objective

Close a service-planning read-side consistency gap where a corrupted local
database import could expose padded but otherwise known planning status text
even though ordinary repository writes normalize status inputs.

## Changes

- Normalized persisted service-schedule status text while listing planning
  rows.
- Added regression coverage that inserts a padded known status directly into
  SQLite with check constraints bypassed and proves repository list reads return
  the canonical status.
- Updated changelog, roadmap, and service-planning documentation.

## Boundaries

- No storage migration, release version bump, tag, Rust API route, or new
  planning workflow was added.
- Unknown persisted statuses remain rejected; only known statuses with
  surrounding whitespace are normalized in memory.

## Validation

- Targeted read-side regression:
  `$env:PYTHONPATH='python'; py -m unittest python.tests.test_sqlite_repositories.ProjectRepositoryScheduleTests.test_repository_normalizes_service_schedule_status_on_list`
  - passed with `1` test.
- Service-planning repository regression module:
  `$env:PYTHONPATH='python'; py -m unittest python.tests.test_sqlite_repositories.ProjectRepositoryScheduleTests`
  - passed with `37` tests.
- Python package compilation:
  `py -m compileall python/emc_locus`
  - passed.
- Rust test suite:
  `cargo test`
  - passed with `46` agent tests and `167` core tests.
