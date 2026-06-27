# 2026-06-27 GUI Bootstrap Session

## Intent

Start wiring the static operator console to local Python services without
requiring a web server or internet access.

## Changes

- Added `python/emc_locus/gui_bootstrap.py`.
- Added a Python exporter that maps project, metrology, test-definition,
  measurement-data, and update-catalog SQLite repositories into the GUI data
  contract.
- Added generated `apps/gui-shell/bootstrap.js` for direct browser loading.
- Updated the GUI to load `bootstrap.js` before falling back to embedded
  fixture data.
- Added `MeasurementDataRepository.list_datasets()` for full dataset export.
- Added a repository-backed Python test that seeds local SQLite databases and
  verifies the generated GUI payload.
- Updated changelog, roadmap, product objectives, app README, and recurring
  backlog.

## Validation Evidence

- `py -m compileall python\emc_locus python\tests`: passed.
- SQLite migration validation: passed with `measurement_data` at version 2.
- Python unittest discovery with `python` added to `sys.path`: 10 tests passed.
- `cargo fmt --check`: passed.
- `cargo test`: 119 Rust tests passed.
- Bundled `node.exe --check apps\gui-shell\app.js`: passed.
- Bundled `node.exe --check apps\gui-shell\bootstrap.js`: passed.
- `git diff --check`: passed.

## Next Work

- Add local write actions behind project stage movement and dataset retention
  workflows.
- Add a one-command refresh flow that regenerates `bootstrap.js` from selected
  local repositories.
