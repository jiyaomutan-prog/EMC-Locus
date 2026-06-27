# 2026-06-27 Instrument Observation Persistence Session

## Intent

Persist instrument command/response observations in the local measurement-data
repository so offline execution evidence can survive beyond the in-memory Rust
runtime.

## Changes

- Added measurement-data migration v6 for `instrument_observations`.
- Stored project, campaign, run, instrument, transport, endpoint, command,
  response, success state, exchange attempts, timestamp, and raw payload JSON.
- Added Python repository APIs to record observations, list observations by run,
  list by run/instrument, and retrieve latest observations.
- Fed Qt runtime bootstrap rows from the latest local observation records when a
  measurement-data repository is available.
- Updated repository/bootstrap tests, storage-schema notes, and changelog.

## Validation Evidence

- SQLite migration validation: passed with measurement_data=6, metrology=1,
  projects=1, sync=1, test_definitions=1, update_catalog=2.
- Targeted Python tests for observation persistence and bootstrap runtime rows:
  passed.
- Python unittest discovery: 21 tests passed.
- `python -m compileall -q python\emc_locus python\tests`: passed.
- `py -m py_compile apps\qt-console\main.py`: passed with bundled Python.
- `node --check apps\gui-shell\app.js`: passed with bundled Node.js.
- `cargo fmt --check`: passed.
- `cargo test -q`: 139 Rust tests passed.
- `git diff --check`: passed.

## Next Work

- Map Rust `InstrumentObservation` values into this SQLite repository boundary.
- Add Qt runtime filters once multiple active runs/instruments exist.
- Include persisted observation IDs in traceability report exports.
