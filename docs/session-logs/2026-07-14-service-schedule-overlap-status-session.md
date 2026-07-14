# 2026-07-14 - Service Schedule Overlap Status Session

## Objective

Continue the bounded service-schedule hardening stream by fixing an imported-row
edge case in the repository overlap guard, without starting a new product
vertical.

## Work Completed

- Revalidated an imported overlap candidate's persisted status with the
  canonical service-schedule status vocabulary before reporting an operator or
  location conflict.
- Added a regression that inserts a constraint-bypassed planning row with an
  unknown `paused` status, then verifies a later overlapping insert surfaces
  the status corruption instead of an ordinary resource reservation.
- Updated the changelog, roadmap, and service-planning documentation to record
  the repository behavior.

## Validation Evidence

- `PYTHONPATH=python py -m unittest python.tests.test_sqlite_repositories.ProjectRepositoryScheduleTests.test_repository_revalidates_imported_overlap_status_before_rejecting`: pass.
- `PYTHONPATH=python py -m unittest python.tests.test_sqlite_repositories.ProjectRepositoryScheduleTests`: pass, 78 tests.
- `py -m compileall python\emc_locus`: pass.
- `PYTHONPATH=python py -m unittest discover -s python\tests`: pass, 176 tests.
- `cargo test`: pass, 52 agent tests and 210 core tests.
- `git diff --check`: pass; Git reported only existing CRLF conversion warnings.

## Remaining Limits

- The overlap guard still relies on repository insertion paths; callers should
  keep using repository APIs rather than writing `service_schedule_items`
  directly.
- This session did not change scheduling scope, station binding, or runtime
  execution behavior.
