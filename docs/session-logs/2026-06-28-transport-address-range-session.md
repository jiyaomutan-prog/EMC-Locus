# 2026-06-28 Transport Address Range Session

## Intent

Resolve an existing validation gap in transport endpoint parsing before adding
native serial or VISA IO, so invalid numeric endpoint values fail at the parser
boundary.

## Changes

- Rejected zero-valued TCP socket ports for VISA-style TCP/IP resources.
- Rejected GPIB primary and secondary addresses outside the 0-30 range.
- Added Rust regression tests for zero TCP socket ports and out-of-range GPIB
  addresses.
- Updated changelog and instrument-control architecture notes.

## Validation Evidence

- Targeted Rust test `cargo test visa_resource_address -- --nocapture` with
  `C:\Users\gtrai\.cargo\bin\cargo.exe`: 2 tests passed.
- Targeted Rust test `cargo test tcp_ip_socket_target -- --nocapture` with
  `C:\Users\gtrai\.cargo\bin\cargo.exe`: 2 tests passed.
- `py -m compileall python/emc_locus`: passed.
- `cargo fmt --check` with `C:\Users\gtrai\.cargo\bin\cargo.exe`: passed.
- `cargo test` with `C:\Users\gtrai\.cargo\bin\cargo.exe`: 142 Rust tests
  passed.

## Next Work

- Continue toward guarded native serial or VISA IO after transport endpoint
  validation remains stable.
