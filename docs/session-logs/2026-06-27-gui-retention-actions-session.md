# 2026-06-27 GUI Retention Actions Session

## Intent

Add local dataset-retention actions behind the static GUI workflow so reviewed
deletion decisions can be recorded offline and reflected in refreshed console
data.

## Changes

- Added `dataset-retention` action to `python/emc_locus/gui_actions.py`.
- Mapped request, approval, rejection, and deletion marking actions to
  measurement-data retention statuses.
- Reused `MeasurementDataRepository.record_retention_event` so transition rules
  and evidence storage stay centralized.
- Added optional `bootstrap.js` regeneration after retention actions.
- Added tests for retention action recording, audit-event reference retention,
  and refreshed bootstrap output.
- Updated GUI README, roadmap, product objectives, changelog, and recurring
  backlog.

## Validation Evidence

- `py -m compileall python\emc_locus python\tests`: passed.
- `python -m emc_locus.gui_actions dataset-retention`: passed through the
  package entrypoint using a temporary measurement-data database.
- SQLite migration validation: passed with `measurement_data` at version 2.
- Python unittest discovery with `python` added to `sys.path`: 13 tests passed.
- `cargo fmt --check`: passed.
- `cargo test`: 119 Rust tests passed.
- Bundled `node.exe --check apps\gui-shell\app.js`: passed.
- `git diff --check`: passed.

## Next Work

- Add update-management local actions for validation and install evidence.
- Continue toward IO-backed instrument adapter implementations.
