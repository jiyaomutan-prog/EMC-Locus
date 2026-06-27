# 2026-06-27 Qt Runtime Table Contract Session

## Intent

Prepare the Qt operator console for instrument-runtime work without faking
hardware control or inventing runtime state.

## Changes

- Added a Qt console `Runtime` table model.
- Defined runtime columns for instrument, transport, endpoint, state, and last
  observation.
- Kept the table empty unless runtime data is provided.
- Added tests for the runtime table contract.
- Updated Qt console documentation and changelog.

## Validation Evidence

- `py -m py_compile apps\qt-console\main.py`: passed.
- `py -m compileall -q python\emc_locus python\tests`: passed.
- Python unittest discovery: 20 tests passed.
- `cargo fmt --check`: passed.
- `cargo test -q`: 137 Rust tests passed.
- SQLite migration validation: passed with measurement_data=5, metrology=1,
  projects=1, sync=1, test_definitions=1, update_catalog=2.
- `node --check apps\gui-shell\app.js`: passed.
- `git diff --check`: passed.

## Next Work

- Feed the runtime table from real instrument-runtime observations.
- Add a dedicated runtime workspace once command execution is wired.
- Connect Qt action intents to audited Python command handlers.
