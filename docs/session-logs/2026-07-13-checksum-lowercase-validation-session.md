# 2026-07-13 - Checksum Lowercase Validation Session

## Objective

Repair a checksum validation inconsistency before starting new `0.14.0`
physical-asset scope.

## Work Completed

- Found that checksum validation text required canonical lowercase SHA-256
  evidence, while the Rust predicates accepted uppercase hexadecimal digits.
- Tightened driver-profile model-link checksum validation for
  `supported_model_definition_checksum`.
- Tightened engineering-curve `source_checksum` validation.
- Added Rust regression coverage for uppercase checksum rejection in both
  domains.
- Updated `CHANGELOG.md`, `docs/domain/driver-profile.md`, and
  `docs/domain/engineering-curves.md` to document the lowercase checksum
  contract.

## Validation Notes

- `cargo test -p emc-locus-core rejects_uppercase_driver_model_checksum`:
  passed.
- `cargo test -p emc-locus-core rejects_uppercase_curve_source_checksum`:
  passed.
- `cargo fmt`: passed and kept Rust formatting consistent.
- `py -m compileall python/emc_locus`: passed.
- `cargo test`: passed (`48` agent tests and `201` core tests).
- `cargo fmt --check`: passed.
- `git diff --check`: passed, with expected Windows line-ending warnings only.

## Limits

This session only fixes checksum evidence validation. It does not change the
SQLite schema, bump `VERSION`, create a release tag, implement physical assets,
station wiring, acquisition, FFT, reports, RBAC, or central sync.
