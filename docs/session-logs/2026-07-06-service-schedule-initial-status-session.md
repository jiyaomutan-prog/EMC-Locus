# 2026-07-06 - Service Schedule Initial Status Session

## Objective

Close the remaining service-planning workflow bypass where a new planning row
could be inserted directly with a non-initial status even though later status
updates are sequential and audited.

## Changes

- Added an explicit initial service-schedule status rule: new rows must start as
  `planned`.
- Applied the rule at the project repository validation boundary so direct
  Python callers and GUI/CLI actions share the same guard.
- Limited the service-schedule creation CLI and Qt form choices to `planned`;
  the separate status-update action still exposes confirmation, start,
  completion, and cancellation transitions.
- Added regression coverage for direct repository inserts, local action calls,
  and Qt form choices.
- Updated changelog, roadmap, and service-planning documentation.

## Boundaries

- No storage migration, release version bump, tag, Rust API route, campaign
  execution feature, or new operator workflow was added.
- Existing audited status transitions remain the path for moving from
  `planned` to `confirmed`, `in_progress`, `completed`, or `cancelled`.

## Validation

- Targeted initial-status regressions:
  `$env:PYTHONPATH='python'; py -m unittest python.tests.test_sqlite_repositories.ProjectRepositoryScheduleTests.test_repository_rejects_non_planned_service_schedule_initial_status python.tests.test_sqlite_repositories.GuiActionTests.test_schedule_service_item_rejects_non_planned_initial_status python.tests.test_qt_console.QtConsoleTests.test_builds_operator_form_specs_for_local_write_repositories`
  - passed with `3` tests.
- Broader service-planning and Qt form regressions:
  `$env:PYTHONPATH='python'; py -m unittest python.tests.test_sqlite_repositories.ProjectRepositoryScheduleTests python.tests.test_sqlite_repositories.GuiActionTests.test_schedule_service_item_records_planned_test_and_refreshes_bootstrap python.tests.test_sqlite_repositories.GuiActionTests.test_schedule_service_item_rejects_non_planned_initial_status python.tests.test_sqlite_repositories.GuiActionTests.test_update_service_schedule_status_records_audit_and_refreshes_bootstrap python.tests.test_qt_console.QtConsoleTests.test_builds_operator_form_specs_for_local_write_repositories python.tests.test_qt_console.QtConsoleTests.test_qt_form_action_schedules_service_and_creates_category_without_pyside`
  - passed with `35` tests after updating older regression setup to reach
    non-initial states through public status transitions.
- Python package compilation:
  `py -m compileall python/emc_locus`
  - passed.
- Rust test suite:
  `cargo test`
  - passed with `46` agent tests and `167` core tests.
- Diff whitespace check:
  `git diff --check`
  - passed; Git reported only Windows CRLF conversion warnings.
