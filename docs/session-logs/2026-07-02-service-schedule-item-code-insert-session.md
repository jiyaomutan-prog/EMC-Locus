# 2026-07-02 - Service Schedule Item Code Insert Session

## Objective

Close a small test-evidence gap around blank service-schedule planning codes on
repository inserts.

## Changes

- Added explicit repository regression coverage proving blank `item_code`
  values are rejected before a schedule row can reach SQLite constraints.
- Documented the planning-code insert guard alongside the existing status-update
  guard.
- Updated changelog and roadmap evidence for the service-planning contract.

## Boundaries

- No behavior change, storage migration, Rust service-planning route, release
  version bump, or tag was added.

## Validation

- Targeted repository schedule tests:
  `$env:PYTHONPATH='python'; py -m unittest python.tests.test_sqlite_repositories.ProjectRepositoryScheduleTests`
  - passed with `13` tests.
- `py -m compileall python/emc_locus` - passed.
- `$env:PYTHONPATH='python'; py -m unittest discover -s python\tests` -
  passed with `96` tests.
- `cargo test` - passed with `44` agent tests and `162` core tests.
