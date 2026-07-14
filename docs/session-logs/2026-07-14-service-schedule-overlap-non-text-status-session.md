# 2026-07-14 - Service Schedule Overlap Non-Text Status Session

## Objective

Continue the bounded service-schedule hardening stream by closing one imported
row edge case in the overlap guard, without expanding scheduling scope.

## Work Completed

- Included non-text persisted statuses in service-schedule overlap-candidate
  selection so a constraint-bypassed active row is revalidated by the canonical
  status gate instead of being skipped before a conflicting insert.
- Added a regression that inserts an overlapping imported row with a BLOB
  status and verifies the later insert reports `status must be text` without
  creating a second planning row.
- Updated the changelog, roadmap, and service-planning documentation with the
  repository behavior.

## Validation Evidence

- `PYTHONPATH=python py -m unittest python.tests.test_sqlite_repositories.ProjectRepositoryScheduleTests.test_repository_revalidates_non_text_imported_overlap_status`: pass.
- `PYTHONPATH=python py -m unittest python.tests.test_sqlite_repositories.ProjectRepositoryScheduleTests`: pass, 79 tests.
- `py -m compileall python\emc_locus`: pass.
- `PYTHONPATH=python py -m unittest discover -s python\tests`: pass, 177 tests.
- `cargo test`: pass, 52 agent tests and 210 core tests.

## Remaining Limits

- The overlap guard still evaluates rows through repository insert paths; direct
  SQLite writers remain outside the supported application boundary.
- This session did not add station binding, resource calendars, or runtime
  execution behavior.
