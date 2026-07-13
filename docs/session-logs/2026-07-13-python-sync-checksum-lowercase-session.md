# 2026-07-13 - Python Sync Checksum Lowercase Session

## Objective

Repair a Python sync checksum validation inconsistency before starting new
`0.14.0` physical-asset scope.

## Work Completed

- Found that the shared Python `require_sha256_checksum` predicate accepted
  uppercase hexadecimal checksum evidence while current checksum contracts
  require canonical lowercase `sha256:<64 hex>` values.
- Tightened Python sync operation payload checksum and entity snapshot checksum
  validation to reject uppercase hexadecimal characters before persistence.
- Added repository regression coverage for uppercase operation payload and
  snapshot checksums.
- Updated `CHANGELOG.md`, `docs/offline-first-architecture.md`, and
  `docs/storage-migrations.md` to document the lowercase sync checksum
  contract.

## Validation Notes

- `$env:PYTHONPATH='python'; py -m unittest python.tests.test_sqlite_repositories.SyncRepositoryTests.test_records_operation_journal_with_status_transitions`:
  passed.
- `$env:PYTHONPATH='python'; py -m unittest python.tests.test_sqlite_repositories.SyncRepositoryTests.test_records_entity_snapshots_and_sync_checkpoints`:
  passed.
- `py -m compileall python\emc_locus`: passed.
- `cargo test`: passed (`48` agent tests and `201` core tests).
- `$env:PYTHONPATH='python'; py -m unittest discover -s python\tests`:
  passed (`166` tests).
- `git diff --check`: passed, with expected Windows line-ending warnings only.

## Limits

This session only fixes Python synchronization checksum validation. It does not
change the SQLite schema, bump `VERSION`, create a release tag, implement
physical assets, station wiring, acquisition, FFT, reports, RBAC, or central
sync.
