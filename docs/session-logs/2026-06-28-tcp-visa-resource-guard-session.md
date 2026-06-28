# 2026-06-28 TCP VISA Resource Guard Session

## Intent

Tighten the existing TCP/IP VISA-style endpoint normalization before adding new
serial or VISA IO paths, so malformed socket resources fail clearly instead of
using the default SCPI port.

## Changes

- Changed TCP/IP target normalization to distinguish non-VISA endpoints from
  malformed TCPIP VISA-style resources.
- Kept accepted forms for host-only, explicit-port, `SOCKET`, and `INSTR`
  endpoints.
- Rejected missing VISA TCP hosts, nonnumeric `SOCKET` ports, and unknown
  resource classes.
- Added Rust regression tests for accepted `SOCKET` targets and rejected
  malformed resources.
- Updated changelog and instrument-control architecture notes.

## Validation Evidence

- Targeted Rust test `cargo test tcp_ip_socket_target -- --nocapture` with
  `C:\Users\gtrai\.cargo\bin\cargo.exe`: 2 tests passed.
- `py -m compileall python/emc_locus`: passed.
- `cargo fmt --check` with `C:\Users\gtrai\.cargo\bin\cargo.exe`: passed.
- `cargo test` with `C:\Users\gtrai\.cargo\bin\cargo.exe`: 142 Rust tests
  passed.
- `git diff --check`: passed with Windows CRLF conversion warnings only.

## Next Work

- Continue with guarded serial or VISA IO only after endpoint validation remains
  stable.
