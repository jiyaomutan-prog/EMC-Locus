# 2026-06-27 Qt Runtime Observation Contract Session

## Intent

Make the Qt operator console consume persisted instrument observations as
structured runtime data instead of a single display sentence.

## Changes

- Expanded the Qt runtime table contract to instrument, transport, endpoint,
  state, run, sequence, observation, and exchange-attempt columns.
- Updated local bootstrap runtime rows generated from measurement-data
  observations.
- Added Qt status metrics for runtime failures and maximum exchange attempts.
- Updated Qt model tests and repository-bootstrap tests.
- Updated changelog.

## Validation Evidence

- Targeted Qt/runtime bootstrap tests: passed.
- Python unittest discovery: 21 tests passed.
- `python -m compileall -q python\emc_locus python\tests`: passed.
- `py -m py_compile apps\qt-console\main.py`: passed with bundled Python.
- `node --check apps\gui-shell\app.js`: passed with bundled Node.js.
- `cargo fmt --check`: passed.
- `cargo test -q`: 139 Rust tests passed.
- SQLite migration validation: passed with measurement_data=6, metrology=1,
  projects=1, sync=1, test_definitions=1, update_catalog=2.
- `git diff --check`: passed.

## Next Work

- Add runtime filtering by active run and selected instrument.
- Wire command execution actions to persist observations automatically.
