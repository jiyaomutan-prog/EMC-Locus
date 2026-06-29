# VISA TCP/IP Transport Session

## Objective

Advance the real-instrument adapter stream with a small guarded IO-backed VISA
slice while preserving explicit unavailable-IO behavior for unsupported VISA
interfaces.

## Changes

- Routed VISA TCP/IP resources through the existing guarded TCP socket exchange
  path.
- Preserved explicit unavailable-IO errors for native USB, GPIB, and ASRL VISA
  resources.
- Retained exchange-attempt traceability on VISA TCP/IP exchanges.
- Kept serial endpoint validation strict around whitespace and reserved
  transport prefixes while retaining COM and POSIX serial device forms.
- Updated roadmap, changelog, and instrument-control architecture notes.

## Validation

- Rust targeted VISA adapter tests: passed with 3 tests.
- Python compileall for `python\emc_locus`: passed.
- Rust format check: passed.
- Full Rust test suite: passed with 144 tests.

## Next Work

- Add guarded native serial IO behind the existing serial endpoint settings.
- Select a VISA binding and packaging strategy before adding USB/GPIB/ASRL VISA
  IO.
