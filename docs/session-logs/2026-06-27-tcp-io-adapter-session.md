# 2026-06-27 TCP IO Adapter Session

## Intent

Replace one concrete instrument-control skeleton with a real, testable IO path
without requiring access to laboratory hardware.

## Changes

- Added standard-library TCP/IP exchange support to `TcpIpTransportAdapter`.
- Supported `TCPIP::host::port`, `TCPIP::host`, and `host:port` endpoint forms.
- Applied existing connect timeout, response timeout, and retry policy values.
- Wrote newline-terminated commands and read responses until newline or close.
- Added `InstrumentResponse::received` for transport-originated responses.
- Added a local socket test that verifies command bytes and response capture.
- Kept VISA and serial adapters as explicit unavailable-IO skeletons.
- Updated changelog, roadmap, product objectives, instrument-control notes, and
  recurring backlog.

## Validation Evidence

- `cargo fmt --check`: passed.
- `cargo test`: 120 Rust tests passed.
- `py -m compileall python\emc_locus python\tests`: passed.
- SQLite migration validation: passed with `measurement_data` at version 2.
- Python unittest discovery with `python` added to `sys.path`: 14 tests passed.
- Bundled `node.exe --check apps\gui-shell\app.js`: passed.
- `git diff --check`: passed.

## Next Work

- Expand guarded serial or VISA IO implementations.
- Preserve simulator-first coverage for every concrete adapter.
