# 2026-06-27 SQLite Write APIs Session

## Intent

Broaden the first SQLite adapters from insert/query smoke coverage into
controlled update operations for metrology and project repositories.

## Changes

- Enabled SQLite foreign-key enforcement on every adapter connection.
- Added instrument availability updates.
- Added instrument capability JSON updates.
- Added calibration attachment updates for file references and checksums.
- Added project stage persistence with an audit event in the same transaction.
- Added contract-review item completion/upsert.
- Added contract-review item listing.
- Ran a temporary SQLite smoke test covering metrology updates, project stage
  audit side effects, contract-review item completion, and FK rejection for a
  missing project.
- Updated storage docs, roadmap, changelog, README, objectives, and backlog.

## Validation

- `py -m compileall python\emc_locus` passed.
- SQLite migration validation passed for all five repository domains.
- Python SQLite write/update smoke test passed.
- `cargo fmt --check` passed.
- `cargo test` passed with 110 tests.
- `git diff --check` passed.

## Next

- Add update-catalog persistence APIs for signed bundles and install records.
- Add optimized FFT and interpolation-based resampling.
- Add SQLite adapters for measurement data and test-definition domains.
