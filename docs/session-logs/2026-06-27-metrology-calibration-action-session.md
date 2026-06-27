# 2026-06-27 Metrology Calibration Action Session

## Intent

Add the next practical metrology workflow after asset registration: recording a
new calibration certificate for an existing instrument while preserving
calibration history.

## Changes

- Added `record_metrology_calibration` to the local action layer.
- Added the `record-calibration` CLI command through `emc_locus.actions_cli`.
- Exported the calibration action through the Python package.
- Validated existing instrument presence before inserting a calibration record.
- Validated uncertainty JSON before persistence.
- Refreshed GUI bootstrap data after calibration updates when requested.
- Added tests for successful renewal, bootstrap visibility, missing instrument
  rejection, bad JSON rejection, and absence of partial calibration writes.
- Updated metrology operator documentation and changelog.

## Validation Evidence

- Targeted calibration action tests: 2 tests passed.
- Real CLI scenario with `register-instrument` followed by
  `record-calibration`: passed and returned clean JSON for both actions.
- SQLite migration validation: passed with measurement_data=7, metrology=2,
  projects=1, sync=1, test_definitions=1, update_catalog=2.
- Python unittest discovery: 27 tests passed.
- `python -m compileall -q python\emc_locus python\tests`: passed.
- `python -m py_compile python\emc_locus\gui_actions.py
  python\emc_locus\actions_cli.py python\emc_locus\__init__.py
  python\emc_locus\sqlite_repositories.py apps\qt-console\main.py`: passed.
- `node --check apps\gui-shell\app.js`: passed with bundled Node.js.
- `cargo fmt --check`: passed with `C:\Users\gtrai\.cargo\bin\cargo.exe`.
- `cargo test -q`: 139 Rust tests passed.
- `git diff --check`: passed.

## Next Work

- Add instrument availability and category reassignment actions.
- Add a Qt-facing metrology form/service boundary for these actions.
- Connect calibration history to readiness summaries and report traceability.
