# 2026-07-04 - Service Schedule Orphan List Session

## Objective

Close a small repository read-path gap so imported or corrupted
service-schedule rows cannot be listed when their project reference no longer
resolves.

## Changes

- Updated `ProjectRepository.list_service_schedule_items` to join each planning
  row to its project before returning schedule data.
- Rejected orphan planning rows through a controlled `ValueError` instead of
  exposing schedule blocks without campaign context.
- Preserved the existing returned schedule row shape for valid rows by removing
  the internal project-stage check column before returning dictionaries.
- Added regression coverage that inserts an orphan planning row through a
  foreign-key-disabled import path and proves list reads refuse it.
- Documented the read-path guard in the changelog, roadmap, and
  service-planning notes.

## Boundaries

- No storage migration, release version bump, tag, Rust API route, or new
  service-planning workflow was added.
- The guard does not reject valid historical planning rows whose project has
  moved beyond `test_planning`; it only requires the project reference to
  resolve.

## Validation

- Targeted orphan-list regression:
  `$env:PYTHONPATH='python'; py -m unittest python.tests.test_sqlite_repositories.ProjectRepositoryScheduleTests.test_repository_rejects_orphan_service_schedule_item_on_list`
  - passed.
- Targeted repository schedule tests:
  `$env:PYTHONPATH='python'; py -m unittest python.tests.test_sqlite_repositories.ProjectRepositoryScheduleTests`
  - passed with `20` tests.
