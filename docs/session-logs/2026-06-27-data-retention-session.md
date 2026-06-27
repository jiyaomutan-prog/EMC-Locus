# 2026-06-27 Data Retention Session

## Intent

Add the first enforceable retention controls for immutable measurement datasets
before expanding higher-level workflows.

## Changes

- Added Rust retention primitives for dataset retention status and audited
  retention events.
- Required immutable raw datasets to pass through deletion request and approval
  before being marked deleted.
- Added SQLite measurement-data migration `0002_retention_evidence`.
- Added Python `MeasurementDataRepository` APIs for retention event recording,
  retention history lookup, and filtering by current retention status.
- Added tests for Rust transition rules, Python retention workflow persistence,
  and incremental migration from an existing `measurement_data` database.
- Updated roadmap, product objectives, storage docs, changelog, and recurring
  session backlog.

## Validation Evidence

- `cargo fmt --check`: passed.
- `cargo test`: 119 Rust tests passed.
- `py -m compileall python\emc_locus python\tests`: passed.
- SQLite migration validation: passed with `measurement_data` at version 2.
- Python unittest discovery with `python` added to `sys.path`: 9 tests passed.
- Bundled `node.exe --check apps\gui-shell\app.js`: passed.
- `git diff --check`: passed.

## Next Work

- Wire the GUI shell to local Python repository reads.
- Add IO-backed transport implementations behind the Rust adapter boundary.
- Replace the reference DFT fixture with an optimized FFT backend.
