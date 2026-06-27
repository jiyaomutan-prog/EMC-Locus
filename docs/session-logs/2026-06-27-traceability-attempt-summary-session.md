# 2026-06-27 Traceability Attempt Summary Session

## Intent

Carry instrument exchange-attempt evidence into report-review traceability, so
communication instability remains visible without exposing a full raw command
log in the review view.

## Changes

- Added total exchange attempts to `TraceabilityRunView`.
- Added maximum exchange attempts to `TraceabilityRunView`.
- Derived both values from recorded `InstrumentObservation` entries.
- Added a deterministic adapter fixture test for a three-attempt observation.
- Updated traceability report tests, changelog, roadmap, and domain model notes.

## Validation Evidence

- `cargo fmt --check`: passed.
- `cargo test -q`: 139 Rust tests passed.
- `py -m py_compile apps\qt-console\main.py`: passed with bundled Python.
- `python -m compileall -q python\emc_locus python\tests`: passed.
- Python unittest discovery: 20 tests passed.
- SQLite migration validation: passed with measurement_data=5, metrology=1,
  projects=1, sync=1, test_definitions=1, update_catalog=2.
- `node --check apps\gui-shell\app.js`: passed with bundled Node.js.
- `git diff --check`: passed.

## Next Work

- Add a persisted observation-log repository once the measurement-data schema is
  ready for command/response evidence.
- Surface exchange-attempt summaries in the Qt runtime/review views.
