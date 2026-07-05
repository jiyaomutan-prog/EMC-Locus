# 2026-07-05 - Service Schedule Status Action Session

## Objective

Close the operator-facing gap around service-planning status changes so the
audited repository path is used by local Python/CLI and Qt workflows.

## Changes

- Added `update_service_schedule_status_action` to validate status changes,
  call the audited repository update path, and optionally refresh bootstrap
  data.
- Added the `update-service-schedule-status` CLI command.
- Added a Qt operator form and form execution path for audited planning status
  updates.
- Added regression coverage for the local action, bootstrap refresh, Qt form
  contract, and Qt form dispatch.
- Documented the action in the changelog, roadmap, and service-planning notes.

## Boundaries

- No storage migration, release version bump, tag, Rust API route, or web LAB
  CONSOLE workflow was added.
- The existing repository-level non-audited status update remains available for
  compatibility with direct low-level callers.

## Validation

- Targeted action/form regressions:
  `$env:PYTHONPATH='python'; py -m unittest python.tests.test_sqlite_repositories.GuiActionTests.test_update_service_schedule_status_records_audit_and_refreshes_bootstrap python.tests.test_qt_console.QtConsoleTests.test_builds_operator_form_specs_for_local_write_repositories python.tests.test_qt_console.QtConsoleTests.test_qt_form_action_schedules_service_and_creates_category_without_pyside`
  - passed with `3` tests.
- Qt console syntax check:
  `$env:PYTHONPATH='python'; py -m py_compile apps\qt-console\main.py`
  - passed.
- Python package compilation:
  `py -m compileall python/emc_locus`
  - passed.
- Rust test suite:
  `cargo test`
  - passed with `46` agent tests and `167` core tests.
- Broader Python GUI/Qt regression suites:
  `$env:PYTHONPATH='python'; py -m unittest python.tests.test_sqlite_repositories.GuiActionTests python.tests.test_qt_console.QtConsoleTests`
  - passed with `39` tests.
- Whitespace check:
  `git diff --check`
  - passed.
