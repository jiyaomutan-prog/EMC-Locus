# 2026-07-08 - Service Schedule Project Stage Guard Session

## Objective

Close a service-planning integrity gap where constraint-bypassed imports could
insert planning rows for projects still in quotation or contract review, then
surface those rows through repository list reads or advance them through status
updates before the contract-review gate had reached `test_planning`.

## Changes

- Added a repository-side minimum project-stage validation for persisted
  service-schedule rows consumed by list reads and status updates.
- Kept post-planning project stages valid for existing planning traceability.
- Added regression coverage for pre-planning imported rows on list and update
  paths.
- Updated changelog, roadmap, and service-planning documentation.

## Boundaries

- No storage migration, release version bump, tag, Rust API route, or new
  planning workflow was added.
- The guard rejects corrupted imported rows; it does not rewrite existing
  SQLite planning data.

## Validation

- Targeted pre-planning project-stage regressions:
  `$env:PYTHONPATH='python'; py -m unittest python.tests.test_sqlite_repositories.ProjectRepositoryScheduleTests.test_repository_rejects_pre_planning_service_schedule_item_on_list python.tests.test_sqlite_repositories.ProjectRepositoryScheduleTests.test_repository_rejects_pre_planning_service_schedule_item_on_update`
  - passed.
- Service-planning repository regression module:
  `$env:PYTHONPATH='python'; py -m unittest python.tests.test_sqlite_repositories.ProjectRepositoryScheduleTests`
  - passed with `55` tests.
- Python package compilation:
  `py -m compileall python/emc_locus`
  - passed.
- Rust test suite:
  `cargo test`
  - passed with `46` agent tests and `167` core tests.
- Whitespace check:
  `git diff --check`
  - passed with only existing CRLF conversion warnings.
