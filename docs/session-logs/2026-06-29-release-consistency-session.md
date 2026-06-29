# 2026-06-29 Release Consistency Session

## Objective

Add an automated guard against version drift before continuing the local-agent
vertical slice work.

## Changes

- Added `python/tests/test_release_consistency.py`.
- The test verifies that `VERSION`, workspace Cargo version, `Cargo.lock`
  package versions, Python package version, README current version, and revision
  baseline all agree.
- Bumped the repository to version `0.4.2`.

## Validation

Final pre-commit validation in this session:

```text
$env:PYTHONPATH='python'; python -m unittest python.tests.test_release_consistency
  -> 1 test OK
$env:PYTHONPATH='python'; python -m compileall -q python\emc_locus python\tests
$env:PYTHONPATH='python'; python -m py_compile apps\qt-console\main.py
$env:PYTHONPATH='python'; python -m unittest discover -s python\tests
  -> 49 tests OK
$env:PYTHONPATH='python'; python -c "from pathlib import Path; from emc_locus.migrations import validate_sqlite_migrations; print(validate_sqlite_migrations(Path('storage/sqlite')))"
  -> {'measurement_data': 7, 'metrology': 3, 'projects': 2, 'sync': 3, 'test_definitions': 2, 'update_catalog': 2}
C:\Users\gtrai\.cache\codex-runtimes\codex-primary-runtime\dependencies\node\bin\node.exe --check apps\gui-shell\app.js
cargo fmt --check
cargo test --workspace
  -> 4 agent tests OK and 152 core tests OK
git diff --check
  -> OK, with expected Windows LF/CRLF warnings only
cargo clippy --workspace --all-targets -- -D warnings
  -> not run: cargo-clippy is not installed for toolchain 1.96.0-x86_64-pc-windows-msvc
```

## Next Work

- Extend `emc-locus-agent` with storage init/status/verify commands.
- Keep release/version files under this automated test going forward.
