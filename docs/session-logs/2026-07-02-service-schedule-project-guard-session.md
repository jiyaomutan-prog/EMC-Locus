# 2026-07-02 - Service Schedule Project Guard Session

## Objective

Close the remaining service-planning inconsistency where direct Python
repository callers relied on SQLite foreign-key errors for missing project
references while the GUI/CLI action path returned a controlled planning error.

## Changes

- Added an explicit project-existence check inside
  `ProjectRepository.add_service_schedule_item`.
- Added repository regression coverage for a schedule insert targeting an
  unknown project.
- Updated changelog, roadmap, and service-planning documentation.

## Boundaries

- No storage migration, Rust service-planning route, campaign test
  instantiation, holiday calendar, or release/version bump was added.

## Validation

- Targeted repository schedule tests:
  `$env:PYTHONPATH='python'; py -m unittest python.tests.test_sqlite_repositories.ProjectRepositoryScheduleTests`
  - passed with `7` tests.
- `py -m compileall python/emc_locus` - passed.
- `$env:PYTHONPATH='python'; py -m unittest discover -s python\tests` -
  passed with `90` tests.
- `cargo test` - passed with `44` agent tests and `162` core tests.
