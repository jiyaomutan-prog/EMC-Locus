# 2026-07-12 - Measurement Engineering Replay Session

## Objective

Repair a post-`0.13.0` measurement-engineering idempotence defect before
starting new `0.14.0` physical-asset scope.

## Work Completed

- Found that measurement-engineering audit events stored operation fingerprints
  with the shared `measurement_engineering_revision` entity type, while replay
  checks recalculated fingerprints with aggregate-specific entity types such as
  `scaling_profile_revision`.
- Added a shared repository constant for the measurement-engineering replay
  fingerprint entity type and used it from both audit writes and service replay
  checks.
- Extended the local API measurement-engineering workflow test to replay
  scaling-profile create, draft replacement, submit, and approve operations and
  assert `replayed: true`.
- Updated `CHANGELOG.md` and `docs/equipment-api.md` to record the corrected
  idempotence contract.

## Validation Notes

- `cargo fmt --check`: initially failed on import ordering after the first edit.
- `cargo fmt`: passed and applied the expected Rust formatting.
- `cargo test -p emc-locus-agent local_api_runs_measurement_engineering_workflow -- --nocapture`:
  passed.
- `py -m compileall python\emc_locus`: passed.
- `cargo fmt --check`: passed after formatting.
- `git diff --check`: passed.
- `cargo test`: passed (`48` agent tests and `195` core tests).

## Limits

This session only fixes measurement-engineering operation replay. It does not
change the SQLite schema, bump `VERSION`, create a release tag, or implement
physical assets, station wiring, acquisition, FFT, reports, RBAC, or central
sync.
