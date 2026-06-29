# 2026-06-29 Sync Local Replay Session

## Objective

Add the first concrete replay helper for the local-first sync path: a pending
operation can be materialized as an entity snapshot and marked applied in a
single SQLite transaction.

## Changes

- Added `SyncRepository.apply_pending_operation_snapshot`.
- The helper reads a pending operation, inserts a snapshot for its resulting
  revision, links the snapshot to the source operation, and marks the operation
  applied.
- Added tests for successful replay, non-replay after applied status, payload
  normalization, source-operation linkage, and invalid checksum rejection.
- Updated version, changelog, storage migration docs, architecture audit, and
  revision-control evidence for `0.3.3`.

## Validation

Final pre-commit validation in this session:

```text
$env:PYTHONPATH='python'; python -m py_compile python\emc_locus\sqlite_repositories.py python\tests\test_sqlite_repositories.py
$env:PYTHONPATH='python'; python -m unittest python.tests.test_sqlite_repositories.SyncRepositoryTests
  -> 6 tests OK
$env:PYTHONPATH='python'; python -m compileall -q python\emc_locus python\tests
$env:PYTHONPATH='python'; python -m py_compile apps\qt-console\main.py
$env:PYTHONPATH='python'; python -m unittest discover -s python\tests
  -> 44 tests OK
$env:PYTHONPATH='python'; python -c "from pathlib import Path; from emc_locus.migrations import validate_sqlite_migrations; print(validate_sqlite_migrations(Path('storage/sqlite')))"
  -> {'measurement_data': 7, 'metrology': 3, 'projects': 2, 'sync': 3, 'test_definitions': 2, 'update_catalog': 2}
C:\Users\gtrai\.cache\codex-runtimes\codex-primary-runtime\dependencies\node\bin\node.exe --check apps\gui-shell\app.js
cargo fmt --check
cargo test -q
  -> 152 tests OK
git diff --check
  -> OK, with expected Windows LF/CRLF warnings only
```

## Next Work

- Add replay selection APIs for batches of pending operations.
- Add deterministic conflict fixture generation from diverging snapshots.
- Move replay execution into a local Rust agent boundary once the agent exists.
