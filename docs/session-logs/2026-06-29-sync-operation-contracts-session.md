# 2026-06-29 Sync Operation Contracts Session

## Objective

Start the local-first synchronization foundation without rewriting the current
Python/SQLite/Qt prototype: define Rust contract value objects, add a durable
SQLite operation journal, and expose transitional Python APIs for replayable
operation records.

## Changes

- Added Rust `contracts` value objects for schema versions, stable ids, entity
  revisions, UTC timestamps, full SHA-256 content checksums, object manifests,
  entity snapshots, and change operations.
- Added sync SQLite migration `0002_operation_journal.sql` with operation id,
  domain, entity, operation kind, base/resulting revision, actor/device,
  correlation id, normalized payload JSON, SHA-256 payload checksum, and replay
  status columns.
- Added Python `SyncRepository` APIs to record, count, get, list, apply, and
  fail operation-journal rows.
- Updated architecture, migration, core-structure, changelog, README, and
  revision-control documentation for release `0.3.1`.

## Validation

Final pre-commit validation in this session:

```text
$env:PYTHONPATH='python'; python -m py_compile python\emc_locus\sqlite_repositories.py python\tests\test_sqlite_repositories.py apps\qt-console\main.py
$env:PYTHONPATH='python'; python -m compileall -q python\emc_locus python\tests
$env:PYTHONPATH='python'; python -m unittest discover -s python\tests
  -> 42 tests OK
$env:PYTHONPATH='python'; python -c "from pathlib import Path; from emc_locus.migrations import validate_sqlite_migrations; print(validate_sqlite_migrations(Path('storage/sqlite')))"
  -> {'measurement_data': 7, 'metrology': 3, 'projects': 2, 'sync': 2, 'test_definitions': 2, 'update_catalog': 2}
C:\Users\gtrai\.cache\codex-runtimes\codex-primary-runtime\dependencies\node\bin\node.exe --check apps\gui-shell\app.js
cargo fmt --check
cargo test -q
  -> 151 tests OK
git diff --check
  -> OK, with expected Windows LF/CRLF warnings only
```

## Next Work

- Persist entity snapshots and checkpoint cursors.
- Add idempotent replay tests over the operation journal.
- Move one Python write command through the Rust application-service boundary.
- Begin the local agent API that will own migrations and synchronization.
