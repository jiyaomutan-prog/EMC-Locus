# 2026-07-03 - Service Schedule Status Target Existence Session

## Objective

Close a small repository robustness gap where direct Python callers could update
the status of an unknown service-schedule item and receive a silent no-op
result.

## Changes

- Updated `ProjectRepository.update_service_schedule_status` to raise a
  controlled `ValueError` when no planning row matches the requested item code.
- Added repository regression coverage for status updates targeting an unknown
  service-schedule item.
- Documented the controlled status-target existence guard in the changelog,
  roadmap, and service-planning notes.

## Boundaries

- No storage migration, Rust service-planning route, new workflow, release
  version bump, or tag was added.

## Validation

- Targeted repository schedule tests:
  `$env:PYTHONPATH='python'; py -m unittest python.tests.test_sqlite_repositories.ProjectRepositoryScheduleTests`
  - passed with `15` tests.
- `py -m compileall python/emc_locus` - passed.
- `$env:PYTHONPATH='python'; py -m unittest discover -s python\tests` -
  passed with `98` tests.
- `cargo test` - passed with `44` agent tests and `162` core tests.
