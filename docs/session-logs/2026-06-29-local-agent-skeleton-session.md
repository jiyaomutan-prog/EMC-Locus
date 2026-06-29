# 2026-06-29 Local Agent Skeleton Session

## Objective

Introduce the first Rust executable for EMC Locus so the project can move
toward a local agent that owns local storage lifecycle, offline sync, health
checks, and eventually a local API.

## Changes

- Added workspace member `crates/emc-locus-agent`.
- Added a testable `health` command parser and JSON health report renderer.
- Added a minimal `main.rs` that prints health JSON or usage errors.
- Added local-agent documentation and README layout updates.
- Bumped the repository to version `0.4.0` because this is the first executable
  Rust application boundary, not only a library/storage patch.

## Validation

Final pre-commit validation in this session:

```text
cargo test -q -p emc-locus-agent
  -> 4 tests OK
cargo run -q -p emc-locus-agent -- health --storage-root storage
  -> JSON health report OK
$env:PYTHONPATH='python'; python -m compileall -q python\emc_locus python\tests
$env:PYTHONPATH='python'; python -m py_compile apps\qt-console\main.py
$env:PYTHONPATH='python'; python -m unittest discover -s python\tests
  -> 46 tests OK
$env:PYTHONPATH='python'; python -c "from pathlib import Path; from emc_locus.migrations import validate_sqlite_migrations; print(validate_sqlite_migrations(Path('storage/sqlite')))"
  -> {'measurement_data': 7, 'metrology': 3, 'projects': 2, 'sync': 3, 'test_definitions': 2, 'update_catalog': 2}
C:\Users\gtrai\.cache\codex-runtimes\codex-primary-runtime\dependencies\node\bin\node.exe --check apps\gui-shell\app.js
cargo fmt --check
cargo test -q
  -> 4 agent tests OK and 152 core tests OK
git diff --check
  -> OK, with expected Windows LF/CRLF warnings only
```

## Next Work

- Add migration initialization checks to the agent health command.
- Add a read-only local status/query API.
- Route one Python GUI action through the agent or Rust application-service
  boundary.
