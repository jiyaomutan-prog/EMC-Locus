# 2026-06-27 GUI Shell Session

## Intent

Answer the need for a visible GUI direction without jumping ahead of the domain,
storage, metrology, and runtime foundations already under construction.

## Changes

- Added `apps/gui-shell`, a static operator console that can be opened directly
  in a browser.
- Added dashboard coverage for active campaigns, readiness issues, signal
  processing traceability, immutable datasets, and update gates.
- Added views for projects, metrology, test definitions, measurement data, and
  update management.
- Added fixture-driven interactions for search, local/reference mode, project
  selection, and project stage advancement.
- Updated roadmap, product objectives, README, changelog, and recurring session
  backlog to include GUI service wiring as a near-term objective.

## Validation Evidence

- `py -m compileall python\emc_locus python\tests`: passed.
- Python unittest discovery with `python` added to `sys.path`: 7 tests passed.
- SQLite migration validation: passed for measurement data, metrology,
  projects, sync, test definitions, and update catalog.
- `cargo fmt --check`: passed.
- `cargo test`: 116 Rust tests passed.
- Bundled `node.exe --check apps\gui-shell\app.js`: passed.
- GUI HTML reference check for `styles.css` and `app.js`: passed.
- `git diff --check`: passed.

## Next Work

- Replace fixture data with local Python repository reads.
- Keep acquisition execution local-first and functional without internet access.
- Add retention policy hooks for immutable measurement datasets.
- Add guarded IO-backed transport adapters behind the existing Rust boundary.
