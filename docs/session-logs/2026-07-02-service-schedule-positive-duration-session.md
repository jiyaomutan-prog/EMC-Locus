# 2026-07-02 - Service Schedule Positive Duration Session

## Objective

Close a small test-evidence gap around service-planning blocks whose planned
end is not after the planned start.

## Changes

- Added repository regression coverage for zero-duration service schedule
  inserts.
- Documented the positive-duration requirement alongside the existing
  canonical timestamp and one-business-day planning rules.
- Updated changelog and roadmap evidence for the planning guard.

## Boundaries

- No behavior change, storage migration, Rust service-planning route, campaign
  test instantiation, release version bump, or tag was added.

## Validation

- Targeted repository schedule tests:
  `$env:PYTHONPATH='python'; py -m unittest python.tests.test_sqlite_repositories.ProjectRepositoryScheduleTests`
  - passed with `11` tests.
- `py -m compileall python/emc_locus` - passed.
- `$env:PYTHONPATH='python'; py -m unittest discover -s python\tests` -
  passed with `94` tests.
- `cargo test` - passed with `44` agent tests and `162` core tests.
