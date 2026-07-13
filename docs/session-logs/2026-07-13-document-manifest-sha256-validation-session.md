# 2026-07-13 - Document Manifest SHA-256 Validation Session

## Objective

Repair a document checksum validation inconsistency before starting new
`0.14.0` physical-asset scope.

## Work Completed

- Found that attached-document registration and metrology calibration document
  manifests accepted uppercase hexadecimal SHA-256 evidence while adjacent
  checksum contracts now require canonical lowercase digests.
- Tightened agent validation for attached-document `sha256` and calibration
  `document_manifest.sha256` fields to require unprefixed 64-character
  lowercase hexadecimal SHA-256 values.
- Added local API regression coverage proving uppercase document and
  calibration-manifest checksums are rejected before writes.
- Updated `CHANGELOG.md`, `docs/document-api.md`, and `docs/metrology-api.md`
  to document the canonical document digest contract.

## Validation Notes

- `cargo test -p emc-locus-agent local_api_registers_attached_document_with_audit_and_outbox`:
  passed.
- `cargo test -p emc-locus-agent local_api_records_calibration_and_computes_status`:
  passed.
- `cargo fmt`: passed and kept Rust formatting consistent.
- `py -m compileall python/emc_locus`: passed.
- `cargo test`: passed (`48` agent tests and `201` core tests).
- `cargo fmt --check`: passed.
- `git diff --check`: passed, with expected Windows line-ending warnings only.

## Limits

This session only fixes agent-side document checksum validation. It does not
change the SQLite schema, bump `VERSION`, create a release tag, implement
physical assets, station wiring, acquisition, FFT, reports, RBAC, or central
sync.
