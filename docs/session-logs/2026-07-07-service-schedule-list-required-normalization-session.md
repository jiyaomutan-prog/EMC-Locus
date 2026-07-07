# 2026-07-07 - Service Schedule List Required Normalization Session

## Objective

Close a service-planning read-side consistency gap where a corrupted local
database import could expose padded required planning text even though ordinary
repository inserts normalize those fields.

## Changes

- Normalized required service-schedule text while listing persisted planning
  rows.
- Added regression coverage that inserts padded required planning text directly
  into SQLite and proves repository list reads return canonical values.
- Updated changelog, roadmap, and service-planning documentation.

## Boundaries

- No storage migration, release version bump, tag, Rust API route, or new
  planning workflow was added.
- The guard normalizes read data in memory; it does not rewrite imported SQLite
  rows.

## Validation

- Targeted read-side regression:
  `$env:PYTHONPATH='python'; py -m unittest python.tests.test_sqlite_repositories.ProjectRepositoryScheduleTests.test_repository_normalizes_required_service_schedule_text_on_list`
  - passed with `1` test.
- Service-planning repository regression module:
  `$env:PYTHONPATH='python'; py -m unittest python.tests.test_sqlite_repositories.ProjectRepositoryScheduleTests`
  - passed with `36` tests.
- Python package compilation:
  `py -m compileall python/emc_locus`
  - passed.
- Rust test suite:
  `cargo test`
  - passed with `46` agent tests and `167` core tests.
