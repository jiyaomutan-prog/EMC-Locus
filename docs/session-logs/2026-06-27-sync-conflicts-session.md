# 2026-06-27 Sync Conflicts Session

## Intent

Add reviewable synchronization conflict records for split repositories used in
offline field work.

## Changes

- Added synchronization conflict ids.
- Added conflict kinds:
  - concurrent update;
  - deleted in reference;
  - deleted locally;
  - checksum mismatch;
  - schema mismatch.
- Added conflict statuses:
  - open;
  - resolved;
  - deferred.
- Added conflict resolutions:
  - keep local;
  - keep reference;
  - manual merge;
  - accept deletion;
  - defer.
- Added conflict records linking local and reference snapshot ids.
- Prevented double resolution of an already resolved conflict.
- Added tests for id validation, open state, resolution, deferred review, and
  double-resolution rejection.
- Updated offline-first architecture, roadmap, changelog, README, and backlog.

## Validation

- `py -m compileall python\emc_locus`
- `$env:PYTHONPATH='python'; py -c "from pathlib import Path; from emc_locus.migrations import validate_sqlite_migrations; print(validate_sqlite_migrations(Path('storage/sqlite')))"`
- `cargo fmt --check`
- `cargo test` passed with 87 tests.
- `git diff --check`

## Next

- Add report export bundle evidence.
- Expand SQLite adapters beyond smoke operations.
- Add sync application services around conflict records.
