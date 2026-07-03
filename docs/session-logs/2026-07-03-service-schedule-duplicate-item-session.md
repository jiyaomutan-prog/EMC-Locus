# 2026-07-03 - Service Schedule Duplicate Item Session

## Objective

Close a small repository robustness gap where direct Python callers could insert
a duplicate service-schedule item code and receive a raw SQLite uniqueness
error.

## Changes

- Updated `ProjectRepository.add_service_schedule_item` to raise a controlled
  `ValueError` when the requested planning item code already exists.
- Added repository regression coverage proving duplicate planning codes are
  rejected before a second row is written.
- Documented the controlled duplicate-code guard in the changelog, roadmap, and
  service-planning notes.

## Boundaries

- No storage migration, Rust service-planning route, new workflow, release
  version bump, or tag was added.

## Validation

- Targeted repository schedule tests:
  `$env:PYTHONPATH='python'; py -m unittest python.tests.test_sqlite_repositories.ProjectRepositoryScheduleTests`
  - passed with `16` tests.
- `py -m compileall python/emc_locus` - passed.
- `$env:PYTHONPATH='python'; py -m unittest discover -s python\tests` -
  passed with `99` tests.
- `cargo test` - passed with `44` agent tests and `162` core tests.
