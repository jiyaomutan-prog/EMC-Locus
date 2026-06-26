# 2026-06-27 SQLite Query APIs Session

## Intent

Expand the early SQLite adapters beyond write/count smoke operations by adding
basic read APIs for metrology and project records.

## Changes

- Added metrology instrument lookup.
- Added metrology instrument listing.
- Added latest calibration lookup.
- Added project lookup.
- Added project listing.
- Added ordered project audit-event listing.
- Added a shared row-to-dict helper for adapter query results.
- Validated read/write paths with a migration-backed temporary SQLite smoke
  test.
- Updated storage migration docs, roadmap, changelog, README, and backlog.

## Validation

- `py -m compileall python\emc_locus`
- `$env:PYTHONPATH='python'; py -c "from pathlib import Path; from emc_locus.migrations import validate_sqlite_migrations; print(validate_sqlite_migrations(Path('storage/sqlite')))"`
- migration-backed smoke test creating temporary metrology and project SQLite
  databases, inserting and reading one instrument, one calibration, one project,
  and one audit event.
- `cargo fmt --check`
- `cargo test` passed with 90 tests.
- `git diff --check`

## Next

- Add optimized FFT/windowing and resampling execution.
- Add signed update bundle workflow.
- Add broader SQLite write/update APIs.
