# 2026-06-27 Measurement Data Adapter Session

## Intent

Add the first Python SQLite adapter for immutable measurement data and processed
signal artifacts.

## Changes

- Added Python `MeasurementDataRepository`.
- Exported the new adapter from the Python package.
- Added dataset insert/count/get/by-checksum/list-by-run APIs.
- Added signal channel insert/list APIs.
- Added processing graph insert/list APIs.
- Added result artifact insert/list APIs.
- Ran a temporary SQLite smoke test covering raw dataset metadata, channel
  metadata, processing graph metadata, result artifacts, and FK rejection for an
  orphan signal channel.
- Updated storage docs, roadmap, changelog, README, objectives, and backlog.

## Validation

- `py -m compileall python\emc_locus` passed.
- SQLite migration validation passed for all five repository domains.
- Python measurement-data smoke test passed.
- `cargo fmt --check` passed.
- `cargo test` passed with 116 tests.
- `git diff --check` passed.

## Next

- Add sync persistence adapters around conflict action plans.
- Add SQLite adapters for test-definition domains.
- Add IO-backed VISA, TCP/IP, or serial implementations behind the skeletons.
