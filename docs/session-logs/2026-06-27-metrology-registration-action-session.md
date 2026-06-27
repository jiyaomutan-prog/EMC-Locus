# 2026-06-27 Metrology Registration Action Session

## Intent

Move the metrology feature set from passive category display toward a usable
operator workflow by allowing a local station to register a category-linked
instrument and optional initial calibration certificate.

## Changes

- Added `MetrologyRepository.register_instrument` to insert an instrument and
  optional calibration record in one SQLite transaction.
- Added `register_metrology_instrument` to the local action layer.
- Added the `register-instrument` CLI command with validation for category,
  required identity fields, JSON capability/uncertainty payloads, and complete
  certificate data.
- Added `emc_locus.actions_cli` as the clean `python -m` entry point for local
  operator actions.
- Exported the action through the Python package.
- Added tests for successful registration, bootstrap refresh, unknown category
  rejection, incomplete certificate rejection, and non-persistence of rejected
  records.
- Updated the metrology taxonomy documentation and changelog.

## Validation Evidence

- Targeted metrology registration action tests: 2 tests passed.
- Real CLI invocation through `python -m emc_locus.actions_cli
  register-instrument`: passed and returned clean JSON.
- SQLite migration validation: passed with measurement_data=7, metrology=2,
  projects=1, sync=1, test_definitions=1, update_catalog=2.
- Python unittest discovery: 25 tests passed.
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

- Add a Qt-facing form/service boundary for instrument registration.
- Add calibration renewal/update actions for existing instruments.
- Link category capabilities to driver selection and pre-run readiness checks.
