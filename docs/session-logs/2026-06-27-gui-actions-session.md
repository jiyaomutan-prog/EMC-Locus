# 2026-06-27 GUI Actions Session

## Intent

Add the first local write action behind the static GUI workflow while keeping
the console usable offline and directly from local files.

## Changes

- Added `python/emc_locus/gui_actions.py`.
- Added `advance-project` action for audited project stage advancement through
  `ProjectRepository.set_project_stage_with_audit`.
- Added `refresh-bootstrap` action for regenerating GUI data from selected
  local SQLite repositories.
- Exposed action helpers from the Python package.
- Added tests for project advancement, audit event recording, and bootstrap
  regeneration.
- Updated GUI README, roadmap, product objectives, changelog, and recurring
  backlog.

## Validation Evidence

- `py -m compileall python\emc_locus python\tests`: passed.
- `python -m emc_locus.gui_actions refresh-bootstrap`: passed through the
  package entrypoint using a temporary output file.
- SQLite migration validation: passed with `measurement_data` at version 2.
- Python unittest discovery with `python` added to `sys.path`: 12 tests passed.
- `cargo fmt --check`: passed.
- `cargo test`: 119 Rust tests passed.
- Bundled `node.exe --check apps\gui-shell\app.js`: passed.
- `git diff --check`: passed.

## Next Work

- Add dataset retention request, approval, rejection, and deletion actions.
- Add update-management local actions for validation and install evidence.
