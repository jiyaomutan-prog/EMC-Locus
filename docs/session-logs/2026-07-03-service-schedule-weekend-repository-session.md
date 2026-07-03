# 2026-07-03 - Service Schedule Weekend Repository Session

## Objective

Close a small test-evidence gap around the lower-level repository guard for
weekend-only service-schedule blocks.

## Changes

- Added direct `ProjectRepository` regression coverage proving a Saturday
  service-schedule item is rejected before any planning row is written.
- Documented the repository-level weekend guard evidence in the changelog,
  roadmap, and service-planning notes.

## Boundaries

- No behavior change, storage migration, Rust service-planning route, campaign
  instantiation workflow, release version bump, or tag was added.
- The existing shared schedule-block validator remains the single enforcement
  point for GUI/CLI and repository callers.

## Validation

- Targeted repository schedule tests:
  `$env:PYTHONPATH='python'; py -m unittest python.tests.test_sqlite_repositories.ProjectRepositoryScheduleTests`
  - passed with `17` tests.
- `py -m compileall python/emc_locus` - passed.
- `$env:PYTHONPATH='python'; py -m unittest discover -s python\tests` -
  passed with `102` tests.
- `cargo test` - passed with `44` agent tests and `162` core tests.
