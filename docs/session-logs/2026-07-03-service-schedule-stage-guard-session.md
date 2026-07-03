# 2026-07-03 - Service Schedule Stage Guard Session

## Objective

Close a small repository-level validation gap so service-schedule rows cannot
be inserted before a project has reached the planning stage.

## Changes

- Added a `ProjectRepository` guard requiring the referenced project to be in
  `test_planning` before a service-schedule item is inserted.
- Added regression coverage for direct repository callers attempting to plan
  against a project still in `contract_review`.
- Adjusted the bootstrap fixture to create the planning row while the project
  is in `test_planning`, then advance the fixture project to `measuring` for
  the rendered dashboard state.
- Documented the guard in the changelog, roadmap, and service-planning notes.

## Boundaries

- No storage migration, release version bump, tag, Rust API route, or campaign
  instantiation workflow was added.
- The guard is intentionally narrow: it protects new schedule-row insertion and
  does not change status updates for existing planning rows.

## Validation

- Targeted repository schedule tests:
  `$env:PYTHONPATH='python'; py -m unittest python.tests.test_sqlite_repositories.ProjectRepositoryScheduleTests`
  - passed with `18` tests.
- Bootstrap fixture regression:
  `$env:PYTHONPATH='python'; py -m unittest python.tests.test_sqlite_repositories.GuiBootstrapTests.test_builds_bootstrap_from_local_repositories`
  - passed with `1` test.
- Full Python package compilation:
  `py -m compileall python/emc_locus` - passed.
- Full Python test discovery:
  `$env:PYTHONPATH='python'; py -m unittest discover -s python\tests` -
  passed with `103` tests.
- Rust test suite:
  `cargo test` - passed with `44` agent tests and `162` core tests.
