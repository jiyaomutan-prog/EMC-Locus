# 2026-06-28 VISA Resource Shape Session

## Intent

Resolve an existing validation gap in the VISA resource parser before adding
native VISA or serial IO, so malformed descriptors fail early instead of being
accepted by the adapter skeleton.

## Changes

- Made VISA interface prefix parsing reject nonnumeric interface suffixes and
  missing ASRL port indexes.
- Added interface-aware VISA resource shape validation.
- Limited `SOCKET` resources to TCP/IP descriptors with numeric ports.
- Required numeric GPIB primary and secondary addresses.
- Added Rust regression tests for valid TCP/IP socket and secondary GPIB
  resources, plus invalid socket, GPIB, ASRL, and interface-prefix cases.
- Updated changelog and instrument-control architecture notes.

## Validation Evidence

- Targeted Rust test `cargo test visa_resource_address -- --nocapture` with
  `C:\Users\gtrai\.cargo\bin\cargo.exe`: 2 tests passed.
- `py -m compileall python/emc_locus`: passed.
- `$env:PYTHONPATH='python'; py -m unittest discover -s python\tests`: 31 tests
  passed.
- `cargo fmt --check` with `C:\Users\gtrai\.cargo\bin\cargo.exe`: passed.
- `cargo test` with `C:\Users\gtrai\.cargo\bin\cargo.exe`: 142 Rust tests
  passed.

## Next Work

- Continue toward guarded native serial or VISA IO after the resource parsing
  boundary remains stable.
