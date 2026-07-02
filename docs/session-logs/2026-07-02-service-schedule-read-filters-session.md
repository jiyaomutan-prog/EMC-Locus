# 2026-07-02 - Service Schedule Read Filters Session

## Objective

Close a small service-planning read-path ambiguity where direct Python
repository callers could pass blank project filters or unknown status filters
and receive an empty schedule list instead of a controlled validation error.

## Changes

- Normalized `ProjectRepository.list_service_schedule_items` project and status
  filters before building the query.
- Reused the existing required-text and planning-status validators for
  malformed list filters.
- Added repository regression coverage for trimmed readable filters, blank
  project filters, and unknown status filters.
- Updated changelog, roadmap, and service-planning documentation.

## Boundaries

- No storage migration, Rust service-planning route, campaign test
  instantiation, release version bump, or tag was added.

## Validation

- Targeted repository schedule tests:
  `$env:PYTHONPATH='python'; py -m unittest python.tests.test_sqlite_repositories.ProjectRepositoryScheduleTests`
  - passed with `10` tests.
- `py -m compileall python/emc_locus` - passed.
- `$env:PYTHONPATH='python'; py -m unittest discover -s python\tests` -
  passed with `93` tests.
- `cargo test` - passed with `44` agent tests and `162` core tests.
