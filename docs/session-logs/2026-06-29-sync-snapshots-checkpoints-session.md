# 2026-06-29 Sync Snapshots Checkpoints Session

## Objective

Extend the local-first synchronization foundation beyond operation rows by
adding entity snapshots and peer checkpoints. The goal is to prepare replay,
three-way merge, and central synchronization without introducing unsafe
last-write-wins behavior.

## Changes

- Added SQLite sync migration `0003_entity_snapshots.sql`.
- Added `sync_entity_snapshots` for domain/entity/revision baselines with
  normalized payload JSON, optional source operation reference, and full
  SHA-256 snapshot checksums.
- Added `sync_checkpoints` for peer/domain/direction cursors.
- Added Python `SyncRepository` APIs for snapshot count/record/get/latest and
  checkpoint upsert/get/list workflows.
- Added Rust contract test coverage for `EntitySnapshot`.
- Updated release, changelog, storage migration, architecture, and revision
  control documentation for version `0.3.2`.

## Validation

Final pre-commit validation in this session:

```text
$env:PYTHONPATH='python'; python -m unittest python.tests.test_sqlite_repositories.SyncRepositoryTests
  -> 5 tests OK
$env:PYTHONPATH='python'; python -c "from pathlib import Path; from emc_locus.migrations import validate_sqlite_migrations; print(validate_sqlite_migrations(Path('storage/sqlite')))"
  -> {'measurement_data': 7, 'metrology': 3, 'projects': 2, 'sync': 3, 'test_definitions': 2, 'update_catalog': 2}
cargo test -q entity_snapshot_requires_entity_type_and_keeps_revision
  -> 1 test OK
$env:PYTHONPATH='python'; python -m compileall -q python\emc_locus python\tests
$env:PYTHONPATH='python'; python -m py_compile apps\qt-console\main.py
$env:PYTHONPATH='python'; python -m unittest discover -s python\tests
  -> 43 tests OK
C:\Users\gtrai\.cache\codex-runtimes\codex-primary-runtime\dependencies\node\bin\node.exe --check apps\gui-shell\app.js
cargo fmt --check
cargo test -q
  -> 152 tests OK
git diff --check
  -> OK, with expected Windows LF/CRLF warnings only
```

## Next Work

- Add deterministic replay from pending operations into snapshots.
- Add snapshot comparison helpers and conflict fixture generation.
- Route one project/metrology write path through a Rust application service or
  local agent boundary.
