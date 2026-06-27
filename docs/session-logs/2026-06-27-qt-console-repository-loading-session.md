# 2026-06-27 Qt Console Repository Loading Session

## Intent

Reduce the Qt console's dependence on the temporary `bootstrap.js` bridge by
allowing direct loading from split local SQLite repositories.

## Changes

- Added `emc_locus.qt_console_data`.
- Added `build_console_bootstrap_from_repositories`.
- Added Qt console CLI options for local repository paths.
- Kept `bootstrap.js` compatibility when no repository paths are provided.
- Added tests using a real temporary project repository.
- Updated Qt console documentation and changelog.

## Validation Evidence

- `py -m py_compile apps\qt-console\main.py`: passed.
- `py -m compileall -q python\emc_locus python\tests`: passed after
  rerunning separately from unittest to avoid a Windows `.pyc` write race.
- Python unittest discovery: 19 tests passed.
- `cargo fmt --check`: passed.
- `cargo test -q`: 129 Rust tests passed.
- SQLite migration validation: passed with measurement_data=4, metrology=1,
  projects=1, sync=1, test_definitions=1, update_catalog=2.
- `node --check apps\gui-shell\app.js`: passed.
- `git diff --check`: passed.

## Next Work

- Replace `QTableWidget` with proper Qt model/view classes.
- Add direct write actions behind Qt commands.
- Add the first instrument-runtime workspace.
