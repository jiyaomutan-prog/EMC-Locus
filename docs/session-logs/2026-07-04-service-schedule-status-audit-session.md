# 2026-07-04 - Service Schedule Status Audit Session

## Objective

Close a narrow traceability gap so repository-level service-planning status
changes can leave project audit evidence with explicit previous/new status
context.

## Changes

- Added `ProjectRepository.update_service_schedule_status_with_audit` to update
  a planning row status and append a `service_schedule_item_status_updated`
  project audit event in one transaction.
- Reused the same schedule-row existence and orphan-project guards as the
  existing status update path.
- Added regression coverage for the audit sequence, actor, reason, and
  `planned` to `confirmed` status payload.
- Documented the audited status update path in the changelog, roadmap, and
  service-planning notes.

## Boundaries

- No storage migration, release version bump, tag, Rust API route, GUI form, or
  CLI action was added.
- The existing non-audited `update_service_schedule_status` method remains
  available for compatibility.

## Validation

- Targeted audited status update regression:
  `$env:PYTHONPATH='python'; py -m unittest python.tests.test_sqlite_repositories.ProjectRepositoryScheduleTests.test_repository_records_service_schedule_status_update_with_project_audit`
  - passed.
