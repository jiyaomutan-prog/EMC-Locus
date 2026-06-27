# 2026-06-27 Serial Endpoint Settings Session

## Intent

Make the serial transport adapter stricter before adding native serial IO, so
serial instrument control does not rely on ambiguous address strings.

## Changes

- Added serial parity and stop-bit value objects.
- Added structured serial endpoint settings with port, baud rate, data bits,
  parity, and stop bits.
- Accepted `PORT:baud` as default 8N1 framing.
- Accepted explicit framing such as `COM4:9600:7E2`.
- Rejected missing baud rates, zero baud rates, unsupported data bits, and
  unsupported parity markers.
- Stored validated settings in `SerialTransportAdapter`.
- Added Rust tests for parsing, rejection, and adapter exposure.
- Updated changelog, roadmap, and instrument-control architecture notes.

## Validation Evidence

- `cargo fmt --check`: passed.
- `cargo test -q`: 132 Rust tests passed.
- `py -m py_compile apps\qt-console\main.py`: passed.
- `py -m compileall -q python\emc_locus python\tests`: passed.
- Python unittest discovery: 20 tests passed.
- SQLite migration validation: passed with measurement_data=4, metrology=1,
  projects=1, sync=1, test_definitions=1, update_catalog=2.
- `node --check apps\gui-shell\app.js`: passed.
- `git diff --check`: passed.

## Next Work

- Add guarded native serial IO behind this validated endpoint model.
- Expand VISA endpoint parsing and validation.
- Continue Qt command wiring toward audited write actions.
