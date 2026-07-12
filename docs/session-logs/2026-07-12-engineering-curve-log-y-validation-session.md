# 2026-07-12 - Engineering Curve Log-Y Validation Session

## Objective

Repair a measurement-engineering validation gap before starting new `0.14.0`
physical-asset scope.

## Work Completed

- Found that `log_x_linear_y` curves rejected non-positive x values during
  definition validation, while `linear_x_log_y` curves could store
  non-positive dependent values and fail only during evaluation.
- Added definition validation for non-positive dependent values when an
  engineering curve uses `linear_x_log_y` interpolation.
- Added Rust regression coverage for the validation rule.
- Updated `CHANGELOG.md`, `docs/domain/engineering-curves.md`, and
  `docs/equipment-api.md` to document the corrected contract.

## Validation Notes

- `cargo fmt`: passed and kept Rust formatting consistent.
- `cargo test -p emc-locus-core rejects_linear_x_log_y_with_non_positive_curve_values`: passed.
- `py -m compileall python\emc_locus`: passed.
- `cargo test`: passed (`48` agent tests and `196` core tests).
- `cargo fmt --check`: passed.
- `git diff --check`: passed.

## Limits

This session only fixes engineering-curve validation. It does not change the
SQLite schema, bump `VERSION`, create a release tag, implement physical assets,
station wiring, acquisition, FFT, reports, RBAC, or central sync.
