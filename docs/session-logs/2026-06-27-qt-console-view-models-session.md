# 2026-06-27 Qt Console View Models Session

## Intent

Move the first Qt console away from generic bootstrap tables and toward explicit,
testable operator-console view models.

## Changes

- Added `emc_locus.qt_console_models`.
- Added `TableViewModel` and `ConsoleViewModel`.
- Added explicit table columns for projects, metrology, methods, datasets, and
  updates.
- Updated the Qt shell to render view models instead of ad hoc row data.
- Kept the `bootstrap.js` bridge temporary and UI-framework independent.
- Updated Qt console tests and documentation.

## Validation Evidence

- `py -m py_compile apps\qt-console\main.py`: passed.
- `py -m compileall -q python\emc_locus python\tests`: passed.
- Python unittest discovery: 18 tests passed.
- `cargo fmt --check`: passed.
- `cargo test -q`: 129 Rust tests passed.
- SQLite migration validation: passed with measurement_data=4, metrology=1,
  projects=1, sync=1, test_definitions=1, update_catalog=2.
- `node --check apps\gui-shell\app.js`: passed.
- `git diff --check`: passed.

## Next Work

- Replace `QTableWidget` with proper Qt model/view classes.
- Add direct local repository loading from the Qt console.
- Add the first instrument-runtime workspace.
