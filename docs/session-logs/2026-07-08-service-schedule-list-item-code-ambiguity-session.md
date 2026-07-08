# 2026-07-08 - Service Schedule List Item Code Ambiguity Session

## Objective

Close a service-planning read-path gap where imported planning rows with
different stored item codes could normalize to the same canonical planning
identifier and still be exposed through repository list reads.

## Changes

- Added normalized item-code ambiguity detection to service-schedule list reads.
- Added regression coverage proving a project-filtered list rejects imported
  planning rows whose item codes normalize to the same value.
- Updated changelog, roadmap, and service-planning documentation.

## Boundaries

- No storage migration, release version bump, tag, Rust API route, or new
  planning workflow was added.
- The guard detects ambiguous imported rows; it does not rewrite or merge
  existing SQLite planning rows.

## Validation

- Targeted ambiguous list regression:
  `$env:PYTHONPATH='python'; py -m unittest python.tests.test_sqlite_repositories.ProjectRepositoryScheduleTests.test_repository_rejects_ambiguous_service_schedule_item_code_on_list`
  - passed.
- Service-planning repository regression module:
  `$env:PYTHONPATH='python'; py -m unittest python.tests.test_sqlite_repositories.ProjectRepositoryScheduleTests`
  - passed with `51` tests.
- Python package compilation:
  `py -m compileall python/emc_locus`
  - passed.
- Rust test suite:
  `cargo test`
  - passed with `46` agent tests and `167` core tests.
- Whitespace check:
  `git diff --check`
  - passed with only existing CRLF conversion warnings.
