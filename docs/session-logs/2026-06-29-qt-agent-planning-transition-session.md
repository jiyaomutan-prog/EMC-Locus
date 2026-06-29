# 2026-06-29 - Qt Agent Planning Transition Session

## Objective

Make the project transition to `test_planning` available from the Qt operator
forms, not only from the Python action layer.

## Changes

- Added an `advance_project` operator form titled `Passage planning`.
- Routed the form through `advance_project_stage`.
- Preserved the optional `agent_url` path so configured Qt sessions call the
  local Rust agent for the transition.
- Added a Qt-console unit test that verifies the form passes `agent_url` through
  without requiring PySide6.

## Scope Boundaries

- The Qt table data still loads from the existing bootstrap/repository path.
- The remaining metrology, planning, test category, update, and runtime forms
  remain legacy direct SQLite.

## Validation Notes

- Targeted Qt console tests passed with 9 tests.
- Python compile checks passed for `python\emc_locus`, `python\tests`, and
  `apps\qt-console\main.py`.
- `cargo fmt --check` passed.
- `cargo clippy --workspace --all-targets -- -D warnings` passed.
- `cargo test --workspace` passed with 15 agent tests and 152 core tests.
- Python unittest discovery passed with 56 tests.
- SQLite migration validation passed.
- Bundled Node syntax check passed.
