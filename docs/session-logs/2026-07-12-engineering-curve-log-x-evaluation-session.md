# 2026-07-12 - Engineering Curve Log-X Evaluation Session

## Objective

Repair a measurement-engineering evaluation validation gap before starting new
`0.14.0` physical-asset scope.

## Work Completed

- Found that `log_x_linear_y` engineering-curve definitions reject non-positive
  x values, but evaluation requests could still submit a non-positive axis
  value under permissive extrapolation policies.
- Added an evaluation-time guard that rejects non-positive requested x values
  before extrapolation or interpolation can produce non-finite log-domain
  results.
- Added Rust regression coverage for the rejected evaluation request.
- Updated `CHANGELOG.md`, `docs/domain/engineering-curves.md`, and
  `docs/equipment-api.md` to document the corrected evaluation contract.

## Validation Notes

- `cargo fmt`: passed and kept Rust formatting consistent.
- `cargo test -p emc-locus-core rejects_log_x_curve_evaluation_with_non_positive_axis_request`:
  passed.
- `py -m compileall python\emc_locus`: passed.
- `cargo test`: passed (`48` agent tests and `197` core tests).
- `cargo fmt --check`: passed.
- `git diff --check`: passed, with expected Windows line-ending warnings only.

## Limits

This session only fixes engineering-curve evaluation validation. It does not
change the SQLite schema, bump `VERSION`, create a release tag, implement
physical assets, station wiring, acquisition, FFT, reports, RBAC, or central
sync.
