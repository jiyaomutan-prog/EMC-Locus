# 2026-06-27 Metrology Capabilities Action Session

## Intent

Allow a local station to maintain structured instrument capability declarations
for later driver selection, safety limits, DAQ configuration, and readiness
checks.

## Changes

- Added `set_metrology_instrument_capabilities` to the local action layer.
- Added the `set-instrument-capabilities` CLI command through
  `emc_locus.actions_cli`.
- Exported the capability action through the Python package.
- Validated instrument presence before capability updates.
- Validated capability JSON before persistence.
- Refreshed GUI bootstrap data after capability updates when requested.
- Added tests for successful capability replacement, missing instrument
  rejection, invalid JSON rejection, and unchanged state after rejected updates.
- Updated metrology operator documentation and changelog.

## Validation Evidence

- Targeted capability action tests: 2 tests passed.
- Real CLI scenario with `register-instrument` followed by
  `set-instrument-capabilities`: passed and returned clean JSON for both
  actions.
- SQLite migration validation: passed with measurement_data=7, metrology=2,
  projects=1, sync=1, test_definitions=1, update_catalog=2.
- Python unittest discovery: 31 tests passed.
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

- Surface selected capabilities in the Qt metrology detail view.
- Use capabilities to match instruments with driver transports and test-method
  requirements.
- Add category-specific capability templates.
