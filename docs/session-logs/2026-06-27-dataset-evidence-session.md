# 2026-06-27 Dataset Evidence Session

## Intent

Connect accepted measurement-run plans to immutable raw dataset evidence and
instrument observations.

## Changes

- Added a Rust `datasets` module.
- Added dataset references and file references.
- Added SHA-256 checksum value object.
- Added dataset kinds for raw, processed, result, and report artifacts.
- Added immutable raw dataset records.
- Added measurement-run evidence linking:
  - accepted run plan;
  - instrument observations;
  - raw datasets.
- Added validation that a raw dataset must belong to the same run reference as
  the accepted plan.
- Added tests for reference validation, checksum validation, immutable raw
  records, evidence accumulation, and run mismatch rejection.
- Updated domain model, roadmap, changelog, README, core structure, and backlog.

## Validation

- `py -m compileall python\emc_locus`
- `$env:PYTHONPATH='python'; py -c "from pathlib import Path; from emc_locus.migrations import validate_sqlite_migrations; print(validate_sqlite_migrations(Path('storage/sqlite')))"`
- `cargo fmt --check`
- `cargo test` passed with 65 tests.
- `git diff --check`

## Next

- Start numeric signal-processing execution fixtures.
- Add report approval gates for accredited workflows.
- Add persistent adapters for metrology and project repositories.
