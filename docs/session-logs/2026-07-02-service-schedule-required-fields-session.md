# 2026-07-02 - Service Schedule Required Fields Session

## Objective

Close the remaining service-planning bypass where direct Python repository
callers could persist schedule rows with blank operator, location, title, or
equipment context even though the GUI/CLI action rejected those inputs.

## Changes

- Moved required service-schedule field validation into
  `ProjectRepository.add_service_schedule_item`.
- Reused the repository module's `require_non_empty` helper so stored schedule
  rows use trimmed required fields.
- Added a repository regression test for a blank assigned operator.
- Updated changelog, roadmap, and service-planning documentation.

## Boundaries

- No storage migration, holiday calendar, capacity planning, Rust
  service-planning route, or release/version bump was added.

## Validation

- Targeted repository schedule tests:
  `$env:PYTHONPATH='python'; py -m unittest python.tests.test_sqlite_repositories.ProjectRepositoryScheduleTests`
  - passed with `5` tests.
- `py -m compileall python/emc_locus` - passed.
- `$env:PYTHONPATH='python'; py -m unittest discover -s python\tests` -
  passed with `88` tests.
- `cargo test` - passed with `44` agent tests and `162` core tests.
