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

Version `0.8.2` was validated on 2026-06-30 with:

```text
cargo metadata --format-version 1
py -m py_compile apps\qt-console\main.py
C:\Users\gtrai\.cache\codex-runtimes\codex-primary-runtime\dependencies\node\bin\node.exe --check apps\gui-shell\app.js
py -c "from pathlib import Path; html=Path('apps/gui-shell/index.html').read_text(encoding='utf-8'); js=Path('apps/gui-shell/app.js').read_text(encoding='utf-8'); ids=['nav-list','status-strip','view-title','view-summary','search-input','overview-view','space-view','console-grid','shared-backbone','static-guardrails','lab-domain-map','relationship-flow','space-kind','space-group','space-objective','space-handoff','space-guardrail','space-objects','space-actions','space-relations','space-table']; missing=[item for item in ids if f'id=\"{item}\"' not in html and f'#{item}' in js]; assert not missing, missing"
Temporary local HTTP server for apps\gui-shell plus Invoke-WebRequest returned HTTP 200 for /index.html
$env:PYTHONPATH='python'; py -m unittest python.tests.test_release_consistency
git diff --check
git diff --cached --check
```
