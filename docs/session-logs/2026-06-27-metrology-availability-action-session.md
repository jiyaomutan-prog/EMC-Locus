# 2026-06-27 Metrology Availability Action Session

## Intent

Make the metrology registry operational by allowing a local station to mark an
instrument available, reserved, or out of service and immediately reflect that
status in the GUI bootstrap.

## Changes

- Added `set_metrology_instrument_availability` to the local action layer.
- Added the `set-instrument-availability` CLI command through
  `emc_locus.actions_cli`.
- Exported the availability action through the Python package.
- Validated requested status values before persistence.
- Validated existing instrument presence before status changes.
- Refreshed GUI bootstrap data after status updates when requested.
- Added tests for out-of-service bootstrap display, invalid status rejection,
  missing instrument rejection, and unchanged state after rejected updates.
- Updated metrology operator documentation and changelog.

## Validation Evidence

- Targeted availability action tests: 2 tests passed.
- Real CLI scenario with `register-instrument` followed by
  `set-instrument-availability`: passed and returned clean JSON for both
  actions.
- SQLite migration validation: passed with measurement_data=7, metrology=2,
  projects=1, sync=1, test_definitions=1, update_catalog=2.
- Python unittest discovery: 29 tests passed.
- `python -m compileall -q python\emc_locus python\tests`: passed.
- `python -m py_compile python\emc_locus\sqlite_repositories.py
  python\emc_locus\gui_actions.py python\emc_locus\actions_cli.py
  python\emc_locus\__init__.py python\emc_locus\gui_bootstrap.py
  python\emc_locus\qt_console_models.py apps\qt-console\main.py`: passed.
- `node --check apps\gui-shell\app.js`: passed with bundled Node.js.
- `cargo fmt --check`: passed with `C:\Users\gtrai\.cargo\bin\cargo.exe`.
- `cargo test -q`: 139 Rust tests passed.
- `git diff --check`: passed.

## Next Work

- Add a Qt-facing form/service boundary for metrology actions.
- Add capability update actions for existing instruments.
- Feed availability changes into richer pre-run readiness displays.
