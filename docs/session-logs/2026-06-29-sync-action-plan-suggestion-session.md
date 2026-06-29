# 2026-06-29 Sync Action Plan Suggestion Session

## Objective

Add the first suggestion layer for sync conflicts: the system can propose an
audit-visible action plan without marking the conflict resolved or choosing a
winner automatically.

## Changes

- Added `SyncRepository.suggest_conflict_action_plan`.
- Added a conflict-kind mapping that proposes manual merge for checksum or
  concurrent update conflicts, and defer-for-review for deletion/schema
  conflicts.
- The suggestion is idempotent for an existing unapplied action plan.
- Added tests proving the conflict remains open, the suggested plan is reused,
  and manual-merge metadata is recorded.
- Updated version, changelog, storage migration docs, architecture audit, and
  revision-control evidence for `0.3.5`.

## Validation

Final pre-commit validation in this session:

```text
$env:PYTHONPATH='python'; python -m py_compile python\emc_locus\sqlite_repositories.py python\tests\test_sqlite_repositories.py
$env:PYTHONPATH='python'; python -m unittest python.tests.test_sqlite_repositories.SyncRepositoryTests
  -> 8 tests OK
$env:PYTHONPATH='python'; python -m compileall -q python\emc_locus python\tests
$env:PYTHONPATH='python'; python -m py_compile apps\qt-console\main.py
$env:PYTHONPATH='python'; python -m unittest discover -s python\tests
  -> 46 tests OK
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

- Add peer labels to snapshots and conflicts.
- Add batch pending-operation replay selection.
- Move sync orchestration behind a local agent API.
