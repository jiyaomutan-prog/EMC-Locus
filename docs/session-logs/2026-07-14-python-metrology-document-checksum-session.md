# 2026-07-14 - Python Metrology Document Checksum Session

## Goal

Close a remaining Python-side checksum validation gap before starting new
`0.14.0` physical-asset scope.

## Work Completed

- Added a shared Python validator for canonical unprefixed SHA-256 document
  digests.
- Tightened direct metrology SQLite writes for initial calibration records,
  added calibration records, calibration attachment updates, and standalone
  instrument documents.
- Canonicalized Python/Qt local action checksum input so an optional `sha256:`
  prefix is accepted only when the payload is otherwise a lowercase
  64-character digest, and storage remains unprefixed.
- Added regression coverage proving uppercase document checksum evidence is
  rejected before local metrology writes are persisted.
- Updated `CHANGELOG.md`, `docs/roadmap.md`, and `docs/metrology-api.md`.

## Validation Notes

Passed during implementation:

```text
$env:PYTHONPATH='python'; py -m unittest python.tests.test_sqlite_repositories.MetrologyRepositoryTests.test_metrology_repository_rejects_noncanonical_document_checksums
$env:PYTHONPATH='python'; py -m unittest python.tests.test_sqlite_repositories.GuiActionTests.test_metrology_actions_reject_noncanonical_document_checksums
$env:PYTHONPATH='python'; py -m unittest python.tests.test_sqlite_repositories.MetrologyRepositoryTests python.tests.test_sqlite_repositories.GuiActionTests.test_register_metrology_instrument_records_asset_certificate_and_bootstrap python.tests.test_sqlite_repositories.GuiActionTests.test_record_metrology_calibration_updates_existing_asset_and_bootstrap
py -m compileall python/emc_locus
$env:PYTHONPATH='python'; py -m unittest discover -s python\tests
cargo test
git diff --check
```

`git diff --check` passed with the usual Windows CRLF replacement warnings only.

## Limits

This session only fixes Python-side metrology document checksum validation. It
does not change the SQLite schema, bump `VERSION`, create a release tag,
implement physical assets, station wiring, acquisition, FFT, reports, RBAC, or
central sync.
