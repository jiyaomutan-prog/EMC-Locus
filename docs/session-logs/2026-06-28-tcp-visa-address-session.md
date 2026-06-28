# 2026-06-28 TCP VISA Address Session

## Intent

Fix TCP/IP endpoint normalization before expanding serial or VISA IO, so common
VISA-style TCPIP resource strings do not produce incorrect socket targets.

## Changes

- Added TCP/IP target normalization for `TCPIP0::host::port::SOCKET` resources.
- Kept `TCPIP0::host::inst::INSTR` on the default SCPI port instead of treating
  the interface or resource class as host/port values.
- Added Rust tests for local socket exchange through a VISA-style SOCKET
  resource and direct normalization of INSTR, explicit-port, and host-only
  forms.
- Updated changelog and instrument-control architecture notes.

## Validation Evidence

- `py -m compileall python/emc_locus`: passed.
- `cargo fmt --check` with `C:\Users\gtrai\.cargo\bin\cargo.exe`: passed.
- `cargo test` with `C:\Users\gtrai\.cargo\bin\cargo.exe`: 141 Rust tests
  passed.
- `git diff --check`: passed.

## Next Work

- Continue with guarded serial or VISA IO after this TCP/IP address handling
  fix is reviewed.
