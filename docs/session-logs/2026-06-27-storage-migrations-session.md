# 2026-06-27 Storage Migrations Session

## Intent

Turn the storage schema draft into a concrete, versioned starting point while
preserving the product requirement to avoid one opaque central database.

## Changes

- Added split SQLite migration domains:
  - metrology;
  - projects;
  - test definitions;
  - measurement data;
  - update catalog.
- Added one initial migration for each domain.
- Added per-domain `schema_migrations` and `repository_metadata` tables.
- Stored cross-domain links as stable text references rather than SQLite foreign
  keys.
- Added `emc_locus.migrations` with migration discovery and validation helpers.
- Updated storage, architecture, offline-first, roadmap, changelog, README, and
  revision-control documentation.

## Validation

- `py -m compileall python\emc_locus`
- `$env:PYTHONPATH='python'; py -c "from pathlib import Path; from emc_locus.migrations import validate_sqlite_migrations; print(validate_sqlite_migrations(Path('storage/sqlite')))"`

## Next

- Add local snapshot metadata and compatibility checks.
- Build the simulated DAQ source and signal-processing graph fixture.
