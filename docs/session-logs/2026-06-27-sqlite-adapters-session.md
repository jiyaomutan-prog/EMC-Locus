# 2026-06-27 SQLite Adapters Session

## Intent

Prove that the split SQLite migrations can be initialized and used by local
application code without adding external dependencies.

## Changes

- Added Python `SQLiteDomainRepository`.
- Added Python `MetrologyRepository`.
- Added Python `ProjectRepository`.
- Added migration-backed initialization per domain.
- Added metadata reads.
- Added minimal metrology writes:
  - instruments;
  - calibration records.
- Added minimal project writes:
  - projects;
  - project audit events.
- Exported the adapters from `emc_locus`.
- Fixed SQLite connection lifetime so temporary databases close cleanly on
  Windows.
- Updated storage migration docs, roadmap, changelog, README, and backlog.

## Validation

- `py -m compileall python\emc_locus`
- `$env:PYTHONPATH='python'; py -c "from pathlib import Path; from emc_locus.migrations import validate_sqlite_migrations; print(validate_sqlite_migrations(Path('storage/sqlite')))"`
- migration-backed smoke test creating temporary metrology and project SQLite
  databases, inserting one instrument, one calibration, one project, and one
  audit event.
- `cargo fmt --check`
- `cargo test` passed with 79 tests.
- `git diff --check`

## Next

- Connect measurement-run execution to simulated runtime and dataset evidence.
- Add synchronization conflict records for split repositories.
- Expand SQLite adapters beyond smoke operations.
