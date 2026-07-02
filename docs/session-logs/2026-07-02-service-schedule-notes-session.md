# 2026-07-02 - Service Schedule Notes Session

## Objective

Close a small repository robustness gap around optional planning notes.

## Changes

- Added controlled normalization for optional service-schedule `notes` in the
  Python GUI/action path and the project repository path.
- Added repository regression coverage proving `notes=None` persists as an
  empty non-null note instead of raising a raw SQLite constraint error.
- Updated changelog, roadmap, and service-planning documentation.

## Boundaries

- No storage migration, Rust route, version bump, tag, or new planning workflow
  was added.

## Validation

- Targeted repository schedule tests:
  `$env:PYTHONPATH='python'; py -m unittest python.tests.test_sqlite_repositories.ProjectRepositoryScheduleTests`
  - passed with `12` tests.
- `py -m compileall python/emc_locus` - passed.
- `$env:PYTHONPATH='python'; py -m unittest discover -s python\tests` -
  passed with `95` tests.
- `cargo test` - passed with `44` agent tests and `162` core tests.
