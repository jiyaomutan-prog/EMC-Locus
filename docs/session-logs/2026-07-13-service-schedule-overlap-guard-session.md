# 2026-07-13 - Service Schedule Overlap Guard Session

Goal: close a small planning-data integrity gap in business-day service blocks
without adding a new workflow or changing the schema.

## Work Completed

- Added a repository-level overlap guard for new service-schedule rows.
- Rejected active overlapping blocks when the same operator or same location is
  already reserved.
- Kept adjacent blocks valid and allowed reuse of an operator/location after a
  block reaches `completed` or `cancelled`.
- Updated regression tests, `CHANGELOG.md`, `docs/roadmap.md`, and
  `docs/service-planning-and-test-categories.md`.

## Validation Notes

Passed:

```text
$env:PYTHONPATH='python'; py -m unittest python.tests.test_sqlite_repositories.ProjectRepositoryScheduleTests.test_repository_rejects_overlapping_service_schedule_operator python.tests.test_sqlite_repositories.ProjectRepositoryScheduleTests.test_repository_rejects_overlapping_service_schedule_location python.tests.test_sqlite_repositories.ProjectRepositoryScheduleTests.test_repository_allows_adjacent_or_closed_service_schedule_blocks
$env:PYTHONPATH='python'; py -m unittest python.tests.test_sqlite_repositories.ProjectRepositoryScheduleTests
py -m compileall python/emc_locus
$env:PYTHONPATH='python'; py -m unittest discover -s python\tests
cargo test
```

## Limitations

This session only rejects same-operator and same-location overlaps at insert
time. It does not add a conflict calendar UI, resource-capacity modelling,
physical station binding, acquisition runtime, or a release version bump.
