# 2026-07-06 - Service Schedule Status Normalization Session

## Objective

Close a small service-planning input-normalization gap where otherwise valid
status values with surrounding operator whitespace were rejected as unknown
statuses or could risk non-canonical evidence if future callers bypassed UI
choices.

## Changes

- Added a shared service-schedule status normalization helper that trims status
  text and validates it against the controlled status vocabulary.
- Routed repository inserts, list filters, direct status updates, audited
  status updates, and local GUI/CLI actions through the shared helper.
- Added regression coverage for direct repository insert/update/filter paths
  and audited local action status updates.
- Updated changelog, roadmap, and service-planning documentation.

## Boundaries

- No storage migration, release version bump, tag, Rust API route, or new
  operator workflow was added.
- The allowed status vocabulary, initial `planned` rule, terminal-state rule,
  and sequential transition map are unchanged.

## Validation

- Targeted status-normalization regressions:
  `$env:PYTHONPATH='python'; py -m unittest python.tests.test_sqlite_repositories.ProjectRepositoryScheduleTests.test_repository_normalizes_service_schedule_status_text python.tests.test_sqlite_repositories.GuiActionTests.test_update_service_schedule_status_normalizes_status_text`
  - passed with `2` tests.
- Broader service-planning and Qt form regressions:
  `$env:PYTHONPATH='python'; py -m unittest python.tests.test_sqlite_repositories.ProjectRepositoryScheduleTests python.tests.test_sqlite_repositories.GuiActionTests.test_schedule_service_item_records_planned_test_and_refreshes_bootstrap python.tests.test_sqlite_repositories.GuiActionTests.test_schedule_service_item_rejects_non_planned_initial_status python.tests.test_sqlite_repositories.GuiActionTests.test_update_service_schedule_status_records_audit_and_refreshes_bootstrap python.tests.test_sqlite_repositories.GuiActionTests.test_update_service_schedule_status_normalizes_status_text python.tests.test_qt_console.QtConsoleTests.test_builds_operator_form_specs_for_local_write_repositories python.tests.test_qt_console.QtConsoleTests.test_qt_form_action_schedules_service_and_creates_category_without_pyside`
  - passed with `37` tests.
- Python package compilation:
  `py -m compileall python/emc_locus`
  - passed.
- Rust test suite:
  `cargo test`
  - passed with `46` agent tests and `167` core tests.
