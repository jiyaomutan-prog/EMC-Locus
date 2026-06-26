# 2026-06-27 Measurement Execution Binding Session

## Intent

Connect accepted measurement-run plans to simulated instrument execution and raw
dataset evidence.

## Changes

- Added a Rust `execution` module.
- Added measurement execution sessions.
- Ensured a session runtime instrument must be part of the planned equipment.
- Routed simulated command execution into measurement-run evidence.
- Added raw dataset attachment through the execution session.
- Required raw data before finishing an execution session.
- Added tests for unplanned runtime rejection, observation capture, missing raw
  data rejection, and complete evidence return.
- Updated domain model, roadmap, changelog, README, core structure, and backlog.

## Validation

- `py -m compileall python\emc_locus`
- `$env:PYTHONPATH='python'; py -c "from pathlib import Path; from emc_locus.migrations import validate_sqlite_migrations; print(validate_sqlite_migrations(Path('storage/sqlite')))"`
- `cargo fmt --check`
- `cargo test` passed with 83 tests.
- `git diff --check`

## Next

- Add synchronization conflict records for split repositories.
- Add report export bundle evidence.
- Expand SQLite adapters beyond smoke operations.
