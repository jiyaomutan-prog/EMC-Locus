# 2026-07-04 - Service Schedule Project Filter Session

## Objective

Close a narrow repository read-path gap so project-filtered service-schedule
reads cannot silently return an empty planning list when the requested campaign
does not exist.

## Changes

- Updated `ProjectRepository.list_service_schedule_items` to verify an
  explicit `project_code` filter resolves to an existing project before listing
  schedule rows.
- Added regression coverage proving an unknown project filter raises a
  controlled `ValueError` instead of returning an ambiguous empty result.
- Documented the read-filter guard in the changelog, roadmap, and
  service-planning notes.

## Boundaries

- No storage migration, release version bump, tag, Rust API route, or new
  service-planning workflow was added.
- Unfiltered reads keep the existing orphan-row guard, and valid empty
  schedules for existing projects still return an empty list.

## Validation

- Targeted unknown-project filter regression:
  `$env:PYTHONPATH='python'; py -m unittest python.tests.test_sqlite_repositories.ProjectRepositoryScheduleTests.test_repository_rejects_unknown_service_schedule_project_filter`
  - passed.
- Targeted repository schedule tests:
  `$env:PYTHONPATH='python'; py -m unittest python.tests.test_sqlite_repositories.ProjectRepositoryScheduleTests`
  - passed with `21` tests.
- Full Python package compilation:
  `py -m compileall python/emc_locus`
  - passed.
- Rust test suite:
  `cargo test`
  - passed with `46` agent tests and `167` core tests.
