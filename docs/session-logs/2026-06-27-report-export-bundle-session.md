# 2026-06-27 Report Export Bundle Session

## Intent

Link customer-facing report exports to issued report packages, checksums, and
review/approval evidence.

## Changes

- Added report export formats.
- Added report export bundles.
- Required reports to be issued before export evidence can be created.
- Linked export bundles to project code, report number, revision, format, file
  reference, and checksum.
- Preserved reviewer and approver identities in export evidence when available.
- Added tests for export rejection before issue, accredited export evidence, and
  non-accredited export without formal approval.
- Updated domain model, roadmap, changelog, README, and backlog.

## Validation

- `py -m compileall python\emc_locus`
- `$env:PYTHONPATH='python'; py -c "from pathlib import Path; from emc_locus.migrations import validate_sqlite_migrations; print(validate_sqlite_migrations(Path('storage/sqlite')))"`
- `cargo fmt --check`
- `cargo test` passed with 90 tests.
- `git diff --check`

## Next

- Expand SQLite adapters beyond smoke operations.
- Add optimized FFT/windowing and resampling execution.
- Add signed update bundle workflow.
