# 2026-07-12 - Engineering Curve Finite Result Session

## Objective

Repair a measurement-engineering evaluation validation gap before starting new
`0.14.0` physical-asset scope.

## Work Completed

- Found that engineering-curve evaluation checked finite request axes and
  logarithmic input domains, but did not reject non-finite computed dependent
  results after interpolation or permitted extrapolation.
- Added an evaluation-time guard that rejects any computed dependent value that
  is not finite.
- Added Rust regression coverage for an extreme `linear_x_log_y` extrapolation
  that would otherwise return `inf`.
- Updated `CHANGELOG.md`, `docs/domain/engineering-curves.md`, and
  `docs/equipment-api.md` to document the finite-result contract.

## Validation Notes

- `cargo fmt`: passed and kept Rust formatting consistent.
- `cargo test -p emc-locus-core rejects_non_finite_curve_evaluation_result`:
  passed.
- `py -m compileall python\emc_locus`: passed.
- `cargo test`: passed (`48` agent tests and `198` core tests).

## Limits

This session only fixes engineering-curve evaluation result validation. It does
not change the SQLite schema, bump `VERSION`, create a release tag, implement
physical assets, station wiring, acquisition, FFT, reports, RBAC, or central
sync.
