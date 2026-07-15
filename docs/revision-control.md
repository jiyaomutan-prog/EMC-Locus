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

Version `0.20.0` was validated on 2026-07-15 with the CI-equivalent command
set. Its checks cover the same path as GitHub Actions: Rust format, Clippy,
Rust tests, Python compile/tests, SQLite migration validation, LAB CONSOLE
typecheck/lint/unit/build, versioned `dist` verification, Playwright E2E,
release consistency, launcher smoke and whitespace checks. The commands were
run directly with the bundled Node.js runtime available in the workspace.
For `0.20.0`, Playwright adds the laboratory-week workflow to the existing
dossier, method, equipment, measurement-engineering and metrology paths. It
prepares two investigation dossiers, reads both reservations in one week,
filters resources, proves that a conflicting move is refused without dropping
the form, applies a free move, and checks persistence, audit, outbox and return
to the dossier through the real Rust agent. Week and detail views are captured
and reviewed at exactly 1440 x 900 and 1280 x 720. Rust tests cover the week
window, rescheduling state rule, self-exclusion from conflicts, idempotence,
optimistic concurrency, atomic audit/outbox writes and real HTTP persistence
after restart. Python tests cover the agent-backed client and retained
repository compatibility paths.

The explicit command sequence is:

```text
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
py -m compileall python\emc_locus
py -m py_compile apps\qt-console\main.py
$env:PYTHONPATH='python'; py -m unittest discover -s python\tests
$env:PYTHONPATH='python'; py -c "from pathlib import Path; from emc_locus.migrations import validate_sqlite_migrations; print(validate_sqlite_migrations(Path('storage/sqlite')))"
cd apps\lab-console
npm ci
npm run typecheck
npm run lint
npm run test
npm run build
cd ..\..
git diff --exit-code -- apps/lab-console/dist
cd apps\lab-console
npx playwright install chromium
npm run test:e2e
cd ..\..
$env:PYTHONPATH='python'; py -m unittest python.tests.test_release_consistency
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\smoke-launchers.ps1 -SkipQtOffscreen
git diff --check
git diff --cached --check
```

The historical `node --check apps\gui-shell\app.js` check is no longer
applicable to the current product path because `apps/gui-shell` was removed
from the normal release surface when LAB CONSOLE replaced the static shell in
0.10.0. JavaScript/TypeScript validation now runs against
`apps/lab-console/src`, unit tests, Playwright, and the committed production
build.
