# 2026-07-06 - Service Schedule List Required Text Session

## Objective

Close a service-planning read-side gap where a corrupted local database import
could expose a persisted planning row whose required operator/context text was
blank even though ordinary repository inserts reject that shape.

## Changes

- Added repository-side required-text validation while listing service-schedule
  rows.
- Added regression coverage that inserts a blank assigned operator directly
  into SQLite and proves repository list reads reject it.
- Updated changelog, roadmap, and service-planning documentation.

## Boundaries

- No storage migration, release version bump, tag, Rust API route, or new
  planning workflow was added.
- The guard protects Python read paths and UI bootstrap callers from corrupted
  or externally imported rows; ordinary repository inserts already validate the
  same required planning text before persistence.

## Validation

- Targeted read-side regression:
  `$env:PYTHONPATH='python'; py -m unittest python.tests.test_sqlite_repositories.ProjectRepositoryScheduleTests.test_repository_rejects_empty_service_schedule_required_text_on_list`
  - passed with `1` test.
- Service-planning repository regression module:
  `$env:PYTHONPATH='python'; py -m unittest python.tests.test_sqlite_repositories.ProjectRepositoryScheduleTests`
  - passed with `34` tests.
- Python package compilation:
  `py -m compileall python/emc_locus`
  - passed.
- Rust test suite:
  `cargo test`
  - passed with `46` agent tests and `167` core tests.
- Diff whitespace check:
  `git diff --check`
  - passed; Git reported only Windows CRLF conversion warnings.
