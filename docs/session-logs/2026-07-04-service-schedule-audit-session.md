# 2026-07-04 - Service Schedule Audit Session

## Objective

Close a narrow traceability gap so operator-created service-planning rows leave
project audit evidence like other project workflow changes.

## Changes

- Added `ProjectRepository.add_service_schedule_item_with_audit` to insert a
  planning row and `service_schedule_item_planned` audit event in one
  transaction.
- Routed the GUI/CLI `schedule_service_item` action through the audited
  repository path, using the assigned operator as the audit actor.
- Added regression coverage for the repository audit payload and the GUI action
  return value.
- Documented the audit behavior in the changelog, roadmap, and service-planning
  notes.

## Boundaries

- No storage migration, release version bump, tag, Rust API route, or status
  update action was added.
- The existing low-level `add_service_schedule_item` method remains available
  for compatibility with repository tests and direct import paths.

## Validation

- Targeted repository/action tests:
  `$env:PYTHONPATH='python'; py -m unittest python.tests.test_sqlite_repositories.ProjectRepositoryScheduleTests python.tests.test_sqlite_repositories.GuiActionTests`
  - passed with `45` tests.
