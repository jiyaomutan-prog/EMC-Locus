# 2026-06-27 VISA Resource Settings Session

## Intent

Make the VISA adapter stricter before native VISA IO is added, so instrument
resources are classified and rejected early when malformed.

## Changes

- Added VISA interface classification for TCPIP, USB, GPIB, and ASRL resources.
- Added `VisaResourceAddress` with raw address, interface, and resource class.
- Accepted `INSTR` and `SOCKET` resource classes.
- Rejected unsupported interfaces, missing separators, and unsupported resource
  classes.
- Stored validated VISA resource data in `VisaTransportAdapter`.
- Added Rust tests for parsing, rejection, and adapter exposure.
- Updated changelog, roadmap, and instrument-control architecture notes.

## Validation Evidence

- `cargo fmt --check`: passed.
- `cargo test -q`: 135 Rust tests passed.
- `py -m py_compile apps\qt-console\main.py`: passed.
- `py -m compileall -q python\emc_locus python\tests`: passed.
- Python unittest discovery: 20 tests passed.
- SQLite migration validation: passed with measurement_data=4, metrology=1,
  projects=1, sync=1, test_definitions=1, update_catalog=2.
- `node --check apps\gui-shell\app.js`: passed.
- `git diff --check`: passed.

## Next Work

- Select a guarded native VISA binding strategy.
- Add serial IO behind the structured serial endpoint model.
- Continue Qt command wiring toward audited write actions.
