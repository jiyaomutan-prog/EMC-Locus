# 2026-07-06 - Service Schedule List Optional Text Session

## Objective

Close a service-planning read-side consistency gap where a corrupted local
database import could expose blank optional category, method, or notes text
even though ordinary repository inserts normalize those fields.

## Changes

- Normalized optional `test_category_code`, `test_method_code`, and `notes`
  while listing service-schedule rows.
- Added regression coverage that inserts blank and padded optional planning
  text directly into SQLite and proves repository list reads return canonical
  values.
- Updated changelog, roadmap, and service-planning documentation.

## Boundaries

- No storage migration, release version bump, tag, Rust API route, or new
  planning workflow was added.
- The guard normalizes optional read data in memory; it does not rewrite
  imported SQLite rows.

## Validation

- Targeted read-side regression:
  `$env:PYTHONPATH='python'; py -m unittest python.tests.test_sqlite_repositories.ProjectRepositoryScheduleTests.test_repository_normalizes_optional_service_schedule_text_on_list`
  - passed with `1` test.
- Service-planning repository regression module:
  `$env:PYTHONPATH='python'; py -m unittest python.tests.test_sqlite_repositories.ProjectRepositoryScheduleTests`
  - passed with `35` tests.
- Python package compilation:
  `py -m compileall python/emc_locus`
  - passed.
- Rust test suite:
  `cargo test`
  - passed with `46` agent tests and `167` core tests.
