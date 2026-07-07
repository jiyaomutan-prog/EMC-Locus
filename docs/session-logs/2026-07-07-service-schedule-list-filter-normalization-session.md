# 2026-07-07 - Service Schedule List Filter Normalization Session

## Objective

Close a service-planning read-path consistency gap where repository list reads
normalized persisted planning text after fetching rows, but SQL project/status
filters still compared against the raw persisted values.

## Changes

- Matched `ProjectRepository.list_service_schedule_items` project filters and
  project joins against trimmed persisted project codes.
- Matched status filters against trimmed persisted text statuses while keeping
  non-text status storage out of canonical status matches.
- Added regression coverage for a constraint-bypassed planning row whose
  persisted project code and status contain surrounding whitespace but should
  still be returned by canonical filters.
- Updated changelog, roadmap, and service-planning documentation.

## Boundaries

- No storage migration, release version bump, tag, Rust API route, or new
  planning workflow was added.
- The guard normalizes read matching in memory/SQL only; it does not rewrite
  imported SQLite rows.

## Validation

- Service-planning repository regression module:
  `$env:PYTHONPATH='python'; py -m unittest python.tests.test_sqlite_repositories.ProjectRepositoryScheduleTests`
  - passed with `42` tests.
