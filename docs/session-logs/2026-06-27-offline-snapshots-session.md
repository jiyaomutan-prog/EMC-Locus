# 2026-06-27 Offline Snapshots Session

## Intent

Address the remote-reference dependency by modeling the local field package
needed for offline measurement campaigns.

## Changes

- Added stable repository-domain slugs.
- Added repository snapshot identifiers and checksums.
- Added repository snapshots with domain, schema version, checksum, and
  signature evidence.
- Added snapshot requirements for offline-field work.
- Added field repository packages with duplicate-domain detection.
- Added validation for missing snapshots, unsigned snapshots, and incompatible
  schema versions.
- Added tests for all offline package validation paths and a valid complete
  package.
- Updated offline-first architecture, roadmap, changelog, README, core
  structure, and backlog.

## Validation

- `py -m compileall python\emc_locus`
- `$env:PYTHONPATH='python'; py -c "from pathlib import Path; from emc_locus.migrations import validate_sqlite_migrations; print(validate_sqlite_migrations(Path('storage/sqlite')))"`
- `cargo fmt --check`
- `cargo test` passed with 56 tests.
- `git diff --check`

## Next

- Start the simulated instrument runtime with command and observation logs.
- Connect planned measurement runs to raw dataset records and checksums.
- Start numeric signal-processing execution fixtures.
