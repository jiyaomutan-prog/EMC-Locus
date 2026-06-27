# EMC Locus

EMC Locus is an open, auditable platform for EMC test orchestration, metrology
records, campaign traceability, and project data management.

The product goal is to support a full laboratory workflow: quotation, contract
review, test planning, instrument setup, measurement runs, data retention,
technical review, report delivery, and archive.

> Scope note: EMC Locus is an original system based on laboratory needs. It must
> not copy proprietary BAT EMC code, user interface screens, database schemas,
> binary protocols, licensed assets, or confidential documentation.

## Product Pillars

- **Traceability first**: every decision, dataset, instrument, calibration record,
  and report approval must be linked to an audit trail.
- **Metrology aware**: instruments, calibration status, uncertainty inputs, and
  environmental conditions are first-class data.
- **Campaign centered**: a project represents the complete measurement campaign,
  from quote to report delivery.
- **Automation ready**: instrument control should support repeatable procedures
  while keeping human validation points explicit.
- **Standards aligned**: the architecture should help a lab work under
  EN ISO/IEC 17025 practices without claiming certification by itself.

## Initial Architecture Direction

- Rust core for domain rules, traceability invariants, storage contracts, and
  critical instrument-control primitives.
- Python layer for laboratory scripting, adapters, data import/export, analysis
  pipelines, and fast prototyping.
- A first operator-facing GUI shell exists as a static app. It should be wired
  to stable Python and Rust services as those APIs mature.

## Repository Layout

```text
crates/
  emc-locus-core/        Rust domain model and core invariants
apps/
  gui-shell/             Static operator console shell for workflow shaping
docs/
  architecture.md        System boundaries and technical direction
  product-objectives.md  Consolidated product objectives and non-objectives
  core-structure.md      Rust core module map and boundary rules
  domain-model.md        Main laboratory entities and state transitions
  iso-17025-alignment.md Traceability and quality-system mapping
  revision-control.md    Versioning, changelog, tags, and release evidence
  storage-schema.md      First SQLite persistence sketch
  offline-first-architecture.md Local work, split stores, and sync direction
  instrument-control-architecture.md Transport-neutral instrument runtime
  signal-acquisition-analysis.md Time-domain DAQ and signal processing
  session-logs/          Dated development session records
  competitive-analysis/  Public feature baselines and product positioning
  roadmap.md             Incremental delivery plan
python/
  emc_locus/             Python helper package for planning and automation
storage/
  sqlite/                 Versioned SQLite migrations split by domain
```

## First Useful Milestones

1. Expand guarded IO-backed serial and VISA implementations behind the adapter
   skeletons.
2. Add a real optimized FFT implementation behind the backend boundary.
3. Add traceability report views for audit and technical review.

## Development Status

This repository is at foundation stage. The current focus is product framing,
domain modeling, and an implementation skeleton that can grow into tested Rust
and Python modules.

Current software version: `0.1.0`.

Revision tracking uses:

- `VERSION` for the current software version;
- `CHANGELOG.md` for user-visible changes;
- `docs/session-logs/` for dated work records;
- Git commits and future signed tags for release evidence;
- `rust-toolchain.toml` and `Cargo.lock` for Rust build reproducibility.

## Validation

```text
py -m compileall python\emc_locus
$env:PYTHONPATH='python'; py -m unittest discover -s python\tests
$env:PYTHONPATH='python'; py -c "from pathlib import Path; from emc_locus.migrations import validate_sqlite_migrations; print(validate_sqlite_migrations(Path('storage/sqlite')))"
cargo fmt --check
cargo test
node --check apps\gui-shell\app.js
```
