# 2026-06-27 Transport Adapter Boundary Session

## Intent

Create the first serious instrument-control boundary for future VISA, serial,
TCP/IP, USBTMC, and vendor-SDK adapters while keeping the core testable without
hardware.

## Changes

- Added transport endpoint addresses.
- Added a transport adapter trait.
- Added a simulated transport adapter as a conformance fixture.
- Added an adapter-backed runtime that records observations.
- Shared safety-limit enforcement between the legacy simulated runtime and the
  adapter-backed runtime.
- Added transport mismatch errors before exchange.
- Added tests for endpoint validation, simulated exchange, observation logging,
  mismatch rejection, and safety-limit blocking.
- Updated instrument-control architecture, roadmap, changelog, README,
  objectives, core-structure notes, and backlog.

## Validation

- `py -m compileall python\emc_locus` passed.
- SQLite migration validation passed for all five repository domains.
- `cargo fmt --check` passed.
- `cargo test` passed with 105 tests.
- `git diff --check` passed.

## Next

- Add sync application services around split-repository conflict records.
- Add concrete VISA, TCP/IP, or serial adapters behind the transport boundary.
- Add update-catalog persistence APIs for signed bundles and install records.
