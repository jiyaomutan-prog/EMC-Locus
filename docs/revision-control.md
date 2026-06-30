# Revision Control

EMC Locus is intended for laboratory workflows where traceability matters.
Software changes therefore need evidence that is easy to review later.

## Version Source of Truth

The current software version is stored in:

```text
VERSION
```

The Rust workspace package version must match `VERSION`.

## Versioning Policy

Use Semantic Versioning:

```text
MAJOR.MINOR.PATCH
```

During early development:

- `0.1.x` is the foundation line;
- increment `PATCH` for corrections, documentation, and internal refinements;
- increment `MINOR` for meaningful workflow or domain-model additions;
- reserve `1.0.0` for a validated baseline with storage, audit trail,
  instrument simulation, and report package workflows.

## Required Change Evidence

Each meaningful change should leave:

- a Git commit with a descriptive message;
- a `CHANGELOG.md` entry when behavior, workflow, or public documentation changes;
- a dated session log under `docs/session-logs/` for Codex-assisted work;
- validation notes naming the commands run and the result.

## Commit Message Style

Use short imperative commit titles:

```text
Initialize EMC Locus foundation
Add contract review stage gate
Document storage schema draft
```

A commit body should explain:

- what changed;
- why it changed;
- how it was validated;
- any known limitation.

## Tags and Releases

Release tags should use:

```text
vMAJOR.MINOR.PATCH
```

For release candidates:

```text
vMAJOR.MINOR.PATCH-rc.N
```

A release should include:

- matching `VERSION`;
- matching Rust workspace package version;
- updated `CHANGELOG.md`;
- successful validation commands;
- a GitHub release description summarizing the change.

## Reproducibility

Rust reproducibility uses:

- `rust-toolchain.toml` to select the validated Rust toolchain;
- `Cargo.lock` to capture dependency resolution.

Even though the first Rust crate has no third-party dependency, `Cargo.lock`
should remain committed because this repository is an application platform, not
only a reusable library.

## Current Validated Baseline

Version `0.8.1` was validated on 2026-06-30 with:

```text
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
py -m compileall python\emc_locus
py -m py_compile apps\qt-console\main.py
$env:PYTHONPATH='python'; py -m unittest discover -s python\tests
$env:PYTHONPATH='python'; py -c "from pathlib import Path; from emc_locus.migrations import validate_sqlite_migrations; print(validate_sqlite_migrations(Path('storage/sqlite')))"
C:\Users\gtrai\.cache\codex-runtimes\codex-primary-runtime\dependencies\node\bin\node.exe --check apps\gui-shell\app.js
$env:PYTHONPATH='python'; py -m unittest python.tests.test_release_consistency
git diff --check
git diff --cached --check
```
