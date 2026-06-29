# 2026-06-29 Sync Snapshot Conflicts Session

## Objective

Create a deterministic bridge from entity snapshots to synchronization
conflicts. The system should identify divergent snapshots as conflicts without
choosing last-write-wins or applying an implicit merge policy.

## Changes

- Added `SyncRepository.record_snapshot_conflict`.
- The helper loads local and reference snapshots, verifies they describe the
  same domain/entity, skips conflict creation when checksums match, and records
  an open `checksum_mismatch` conflict when they diverge.
- Added tests for divergent snapshots, identical checksums, and mismatched
  entity references.
- Updated version, changelog, storage migration docs, architecture audit, and
  revision-control evidence for `0.3.4`.

## Validation

Final pre-commit validation in this session:

```text
$env:PYTHONPATH='python'; python -m py_compile python\emc_locus\sqlite_repositories.py python\tests\test_sqlite_repositories.py
$env:PYTHONPATH='python'; python -m unittest python.tests.test_sqlite_repositories.SyncRepositoryTests
  -> 7 tests OK
$env:PYTHONPATH='python'; python -m compileall -q python\emc_locus python\tests
$env:PYTHONPATH='python'; python -m py_compile apps\qt-console\main.py
$env:PYTHONPATH='python'; python -m unittest discover -s python\tests
  -> 45 tests OK
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

- Generate action-plan suggestions from snapshot conflict kinds.
- Add central/local peer labels to snapshots.
- Move sync orchestration toward a local agent API.
