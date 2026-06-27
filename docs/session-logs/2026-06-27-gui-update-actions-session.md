# 2026-06-27 GUI Update Actions Session

## Intent

Add local update-management actions behind the static GUI workflow so update
validation and install evidence can be recorded offline.

## Changes

- Added `validate-update` action to `python/emc_locus/gui_actions.py`.
- Added `install-update` action to record installation evidence and optionally
  link accepted validation evidence.
- Reused `UpdateCatalogRepository` validation and install APIs so signature,
  compatibility, offline, and active-measurement gates stay centralized.
- Added optional `bootstrap.js` regeneration after update actions.
- Added tests for accepted validation evidence, install recording, evidence
  linkage, and refreshed bootstrap output.
- Updated GUI README, roadmap, product objectives, changelog, and recurring
  backlog.

## Validation Evidence

- `py -m compileall python\emc_locus python\tests`: passed.
- `python -m emc_locus.gui_actions validate-update`: passed through the package
  entrypoint using a temporary update-catalog database.
- `python -m emc_locus.gui_actions install-update`: passed through the package
  entrypoint using accepted validation evidence.
- SQLite migration validation: passed with `measurement_data` at version 2.
- Python unittest discovery with `python` added to `sys.path`: 14 tests passed.
- `cargo fmt --check`: passed.
- `cargo test`: 119 Rust tests passed.
- Bundled `node.exe --check apps\gui-shell\app.js`: passed.
- `git diff --check`: passed.

## Next Work

- Add IO-backed VISA, TCP/IP, or serial implementations behind the Rust
  adapter skeletons.
- Replace the reference DFT fixture with an optimized FFT backend.
