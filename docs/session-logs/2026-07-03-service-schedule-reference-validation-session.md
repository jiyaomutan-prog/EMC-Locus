# 2026-07-03 - Service Schedule Reference Validation Session

## Objective

Close a small service-planning traceability gap where local action callers could
provide a test category or method reference that was not present in the
test-definition repository.

## Changes

- Updated `schedule_service_item` to validate non-empty test category and
  method references when `test_definitions_db` is provided.
- Added action-level regression coverage for unknown category and method
  references, proving no planning row is written on controlled validation
  errors.
- Documented the action-level reference guard in the changelog, roadmap, and
  service-planning notes.

## Boundaries

- No storage migration, Rust service-planning route, campaign test
  instantiation workflow, release version bump, or tag was added.
- The project repository remains limited to `projects.sqlite`; cross-domain
  taxonomy validation stays in the local action layer where the
  test-definition repository path is available.

## Validation

- Targeted repository schedule tests:
  `$env:PYTHONPATH='python'; py -m unittest python.tests.test_sqlite_repositories.ProjectRepositoryScheduleTests`
  - passed with `16` tests.
- Targeted GUI action tests:
  `$env:PYTHONPATH='python'; py -m unittest python.tests.test_sqlite_repositories.GuiActionTests`
  - passed with `23` tests.
- `py -m compileall python/emc_locus` - passed.
- `$env:PYTHONPATH='python'; py -m unittest discover -s python\tests` -
  passed with `101` tests.
- `cargo test` - passed with `44` agent tests and `162` core tests.
- `git diff --check` - passed with line-ending normalization warnings only.
