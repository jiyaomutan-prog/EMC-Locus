# 2026-07-13 - Service Schedule Padded Overlap Session

Goal: close a small planning-data integrity gap in service-schedule overlap
checks without changing schema or adding a new workflow.

## Work Completed

- Added regression coverage for a constraint-bypassed service-schedule row whose
  planned start/end timestamps and resource fields contain surrounding
  whitespace.
- Updated the repository overlap query to compare `TRIM(planned_start_at)` and
  `TRIM(planned_end_at)`, matching the normalization already applied by list
  reads and status-update validation.
- Updated `CHANGELOG.md`, `docs/roadmap.md`, and
  `docs/service-planning-and-test-categories.md`.

## Validation Notes

Passed:

```text
$env:PYTHONPATH='python'; py -m unittest python.tests.test_sqlite_repositories.ProjectRepositoryScheduleTests.test_repository_rejects_overlapping_service_schedule_with_padded_imported_window
$env:PYTHONPATH='python'; py -m unittest python.tests.test_sqlite_repositories.ProjectRepositoryScheduleTests.test_repository_rejects_overlapping_service_schedule_operator python.tests.test_sqlite_repositories.ProjectRepositoryScheduleTests.test_repository_rejects_overlapping_service_schedule_location python.tests.test_sqlite_repositories.ProjectRepositoryScheduleTests.test_repository_allows_adjacent_or_closed_service_schedule_blocks
$env:PYTHONPATH='python'; py -m unittest python.tests.test_sqlite_repositories.ProjectRepositoryScheduleTests
py -m compileall python/emc_locus
cargo test
```

## Limitations

This session only aligns overlap detection with existing normalization for
whitespace-padded imported rows. It does not add a resource calendar UI,
physical station binding, acquisition runtime, schema migration, release bump,
or tag.
