# 2026-06-27 Transport Attempt Traceability Session

## Intent

Make instrument communication failures easier to diagnose by preserving how many
transport exchange attempts were used for each adapter-backed observation.

## Changes

- Added a default `last_exchange_attempt_count` hook to transport adapters.
- Stored exchange attempt counts on `InstrumentObservation`.
- Recorded adapter attempt counts in `TransportAdapterRuntime` observations.
- Kept simulated runtime observations explicit with one deterministic attempt.
- Tracked TCP/IP retry attempts for successful exchanges and connection
  failures.
- Added Rust tests for successful TCP/IP exchange counts, failed retry counts,
  runtime observations, and simulated observation counts.
- Updated changelog, roadmap, and instrument-control notes.

## Validation Evidence

- `cargo fmt --check`: passed.
- `cargo test -q`: 138 Rust tests passed.
- `py -m py_compile apps\qt-console\main.py`: passed.
- `python -m compileall -q python\emc_locus python\tests`: passed.
- Python unittest discovery: 20 tests passed.
- SQLite migration validation: passed with measurement_data=5, metrology=1,
  projects=1, sync=1, test_definitions=1, update_catalog=2.
- `node --check apps\gui-shell\app.js`: passed with bundled Node.js.
- `git diff --check`: passed.

## Next Work

- Extend retry classification to connected write/read failures.
- Add native serial or VISA IO behind the guarded adapter boundary.
- Persist exchange attempt counts in measurement-data evidence exports.
