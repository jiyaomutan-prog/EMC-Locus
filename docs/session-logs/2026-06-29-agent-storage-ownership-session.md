# 2026-06-29 Agent Storage Ownership Session

## Objective

Move the first local storage lifecycle responsibility into `emc-locus-agent`
for the project vertical slice. This tranche is intentionally limited to the
databases required by the current mission: projects and sync/outbox.

## Changes

- Added `rusqlite` with bundled SQLite to `emc-locus-agent`.
- Added `storage init`, `storage status`, and `storage verify` agent commands.
- The agent discovers project/sync migrations, checks contiguous migration
  versions, creates `projects.sqlite` and `sync.sqlite`, applies missing
  migrations, reports schema versions, verifies foreign keys, and runs SQLite
  integrity checks.
- Added stable JSON reports and structured JSON errors.
- Added Rust tests using temporary storage directories for initialization,
  idempotent re-run, missing status, invalid database detection, and command
  parsing.
- Updated `docs/local-agent.md`, roadmap, changelog, release metadata, and
  revision baseline for `0.4.3`.

## Validation

Final pre-commit validation in this session:

```text
cargo fmt --check
cargo test -q -p emc-locus-agent
  -> 8 tests OK
cargo run -q -p emc-locus-agent -- storage init --storage-root <temp> --migrations-root storage\sqlite
cargo run -q -p emc-locus-agent -- storage status --storage-root <temp> --migrations-root storage\sqlite
cargo run -q -p emc-locus-agent -- storage verify --storage-root <temp> --migrations-root storage\sqlite
  -> project/sync JSON reports OK
$env:PYTHONPATH='python'; python -m unittest python.tests.test_release_consistency
  -> 1 test OK
$env:PYTHONPATH='python'; python -m compileall -q python\emc_locus python\tests
$env:PYTHONPATH='python'; python -m py_compile apps\qt-console\main.py
$env:PYTHONPATH='python'; python -m unittest discover -s python\tests
  -> 49 tests OK
$env:PYTHONPATH='python'; python -c "from pathlib import Path; from emc_locus.migrations import validate_sqlite_migrations; print(validate_sqlite_migrations(Path('storage/sqlite')))"
  -> {'measurement_data': 7, 'metrology': 3, 'projects': 2, 'sync': 3, 'test_definitions': 2, 'update_catalog': 2}
C:\Users\gtrai\.cache\codex-runtimes\codex-primary-runtime\dependencies\node\bin\node.exe --check apps\gui-shell\app.js
cargo test --workspace
  -> 8 agent tests OK and 152 core tests OK
git diff --check
  -> OK, with expected Windows LF/CRLF warnings only
cargo clippy --workspace --all-targets -- -D warnings
  -> not run: cargo-clippy is not installed for toolchain 1.96.0-x86_64-pc-windows-msvc
```

## Next Work

- Add Rust project use cases and SQLite repositories for create/read/review
  operations.
- Add a versioned local API only after the storage and application boundaries
  are testable.
