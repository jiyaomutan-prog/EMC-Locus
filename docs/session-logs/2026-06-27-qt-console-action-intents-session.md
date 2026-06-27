# 2026-06-27 Qt Console Action Intents Session

## Intent

Prepare the Qt console for controlled operator commands without letting the UI
own write rules too early.

## Changes

- Added `OperatorActionIntent`.
- Added project advancement, dataset-retention request, and update-validation
  action intents to the console view model.
- Rendered action intents as Qt header buttons with enabled state and tooltip
  reason.
- Kept audited write execution in the existing Python action layer.
- Added tests for enabled and disabled action-intent states.
- Updated Qt console documentation, GUI technology notes, and changelog.

## Validation Evidence

- `py -m py_compile apps\qt-console\main.py`: passed.
- `py -m compileall -q python\emc_locus python\tests`: passed.
- Python unittest discovery: 20 tests passed.
- `cargo fmt --check`: passed.
- `cargo test -q`: 129 Rust tests passed.
- SQLite migration validation: passed with measurement_data=4, metrology=1,
  projects=1, sync=1, test_definitions=1, update_catalog=2.
- `node --check apps\gui-shell\app.js`: passed.
- `git diff --check`: passed.

## Next Work

- Connect Qt action intents to audited Python command handlers.
- Replace `QTableWidget` with proper Qt model/view classes.
- Add the first instrument-runtime workspace.
