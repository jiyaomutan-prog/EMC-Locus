# 2026-07-03 - Service Schedule Required Text Session

## Objective

Close a small repository robustness gap where direct Python callers could pass
missing required planning text and receive a raw Python attribute error.

## Changes

- Updated the shared `require_non_empty` helper to reject `None` with the same
  controlled `ValueError` used for blank required text.
- Added repository regression coverage proving a missing service-schedule
  `item_code` is rejected before SQLite write attempts.
- Documented the controlled missing-text validation path in the changelog,
  roadmap, and service-planning notes.

## Boundaries

- No storage migration, Rust service-planning route, new workflow, release
  version bump, or tag was added.

## Validation

- Targeted repository schedule tests:
  `$env:PYTHONPATH='python'; py -m unittest python.tests.test_sqlite_repositories.ProjectRepositoryScheduleTests`
  - passed with `14` tests.
- `py -m compileall python/emc_locus` - passed.
- `$env:PYTHONPATH='python'; py -m unittest discover -s python\tests` -
  passed with `97` tests.
- `cargo test` - passed with `44` agent tests and `162` core tests.
