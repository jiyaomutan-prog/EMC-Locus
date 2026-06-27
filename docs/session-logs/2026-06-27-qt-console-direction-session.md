# 2026-06-27 Qt Console Direction Session

## Intent

Correct the GUI technology direction after review: keep the static browser shell
as a workflow prototype, and make Qt desktop the target for the local
measurement-station operator console.

## Changes

- Added `apps/qt-console` as an initial PySide6 bootstrap.
- Reused the existing `bootstrap.js` data contract so the Qt console can start
  from the same local repository export path as the static prototype.
- Kept PySide6 optional at import/test time.
- Added tests for bootstrap parsing and row normalization without requiring Qt.
- Added GUI technology direction documentation.
- Updated README, architecture, product objectives, roadmap, changelog, and the
  static GUI shell README.

## Validation Evidence

- `cargo fmt --check`: passed.
- `cargo test -q`: 129 Rust tests passed.
- `py -m py_compile apps\qt-console\main.py`: passed.
- `py -m compileall -q python\emc_locus python\tests`: passed.
- Python unittest discovery: 18 tests passed.
- SQLite migration validation: passed with measurement_data=4, metrology=1,
  projects=1, sync=1, test_definitions=1, update_catalog=2.
- `node --check apps\gui-shell\app.js`: passed.
- `git diff --check`: passed.

## Next Work

- Replace prototype Qt tables with explicit model/view classes.
- Wire the Qt console directly to application services instead of the temporary
  `bootstrap.js` bridge.
- Add an instrument-runtime workspace once real adapter execution is ready.
