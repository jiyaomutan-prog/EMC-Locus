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
- Rust local agent for local machine health, future SQLite lifecycle ownership,
  offline synchronization, and local API hosting.
- Python layer for laboratory scripting, adapters, data import/export, analysis
  pipelines, and fast prototyping.
- Qt desktop console for the local measurement-station operator experience.
  A static GUI shell exists as a workflow prototype and dashboard mockup, not as
  the long-term UI technology for advanced acquisition work.

## Repository Layout

```text
crates/
  emc-locus-agent/       Rust local agent executable skeleton
  emc-locus-core/        Rust domain model and core invariants
apps/
  gui-shell/             Static operator console shell for workflow shaping
  qt-console/            Qt desktop console bootstrap for measurement stations
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
  gui-technology.md    Qt desktop direction for the operator console
  local-agent.md       Rust local agent command surface and growth path
  project-api.md       Local project API contract
  metrology-api.md     Local metrology API contract
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
2. Add graph-driven execution records for revisioned signal-processing runs.

## Development Status

This repository is at foundation stage. The current focus is product framing,
domain modeling, and an implementation skeleton that can grow into tested Rust
and Python modules.

Current software version: `0.6.6`.

Version `0.6.6` migrates the temporary Qt/Python metrology surface to the Rust
agent when `agent_url` is configured: instruments are listed from the agent,
computed status and readiness are fetched from Rust, instrument registration
and serviceability changes no longer open `metrology.sqlite`, and calibration
events can carry certificate document manifests through the agent path.

Version `0.6.5` adds the first structured metrology readiness and traceability
slice: instrument registration and calibration-event writes now require
operation context and write audit/outbox records atomically, serviceability can
be changed through the agent, and `/api/v1/metrology/readiness` returns ready,
blocking issues, warnings, and per-instrument status.

Version `0.6.4` adds the first agent-backed calibration-event path:
`metrology record-calibration`, calibration-event HTTP routes, certificate
manifest metadata validation, per-instrument due-soon warning days, and a
computed calibration-status endpoint that derives `valid`, `due_soon`,
`expired`, `missing`, `not_required`, or `nonconforming` from the latest event
and the requested check date.

Version `0.6.3` makes the first instrument registry agent-backed: local storage
initialization now creates `metrology.sqlite`, the Rust agent exposes metrology
CLI actions, and the loopback API can register, list, and read instruments
through `/api/v1/metrology/instruments`.

Version `0.6.2` adds the first Rust metrology DTO and repository boundary:
typed instrument/calibration DTOs, checked `metrology.sqlite` opening, service
state schema validation, typed instrument reads, latest-calibration lookup, and
a thin JSON rendering service used by tests. This prepares the agent-backed
metrology registry without yet exposing the full write API.

Version `0.6.1` starts the metrology vertical-slice hardening by separating
instrument serviceability from the legacy availability/reservation field. Legacy
`reserved` instruments remain serviceable by default, out-of-service assets map
to a dedicated service state, and both the Qt/Python surface and static GUI show
service state separately from planning availability.

Version `0.5.0` delivers the first agent-backed project vertical slice:
initialized local project storage, loopback API, project creation,
contract-review completion, transition to test planning, audit events, sync
outbox records, restart/persistence verification, Python client support, and Qt
project forms that can call the local agent when configured. The Qt console also
shows local-agent/storage state and submits agent-backed project forms through a
worker so the operator UI remains responsive.

Version `0.5.1` hardens that slice by making idempotent replays depend on a
canonical operation fingerprint instead of only operation kind and entity id.

Version `0.5.2` replaces hand-built agent response JSON with explicit Serde DTOs
for the project slice, local API, storage reports, health reports, and errors.

Version `0.5.3` splits the project agent into API orchestration, project service,
Serde DTO, and SQLite repository modules while preserving the 0.5.0 vertical
slice behavior.

Version `0.5.4` enforces the project/sync multi-SQLite atomicity policy: storage
initialization uses rollback journal mode, storage status reports journal
compatibility, and project commands refuse incompatible WAL-style configurations.

Version `0.5.5` moves Qt/Python project reads to the local agent when
`agent_url` is configured: project lists, contract-review status, project audit
events, and pending outbox data no longer require direct project SQLite access
for the migrated slice.

Version `0.5.6` adds the first GitHub Actions CI workflow for the same Rust,
Python, SQLite migration, JavaScript, release-consistency, and whitespace checks
used locally before release commits.

Version `0.6.0` finalizes the hardened project vertical slice: strict
idempotence, Serde DTO responses, split Rust project modules, explicit
multi-SQLite atomicity policy, Qt/Python project reads and writes through the
local agent when configured, and CI coverage for the local validation matrix.

Revision tracking uses:

- `VERSION` for the current software version;
- `CHANGELOG.md` for user-visible changes;
- `docs/session-logs/` for dated work records;
- Git commits and future signed tags for release evidence;
- `rust-toolchain.toml` and `Cargo.lock` for Rust build reproducibility.

## Validation

```text
$env:PYTHONPATH='python'; py -m compileall -q python\emc_locus python\tests
$env:PYTHONPATH='python'; py -m py_compile apps\qt-console\main.py
$env:PYTHONPATH='python'; py -m unittest discover -s python\tests
$env:PYTHONPATH='python'; py -c "from pathlib import Path; from emc_locus.migrations import validate_sqlite_migrations; print(validate_sqlite_migrations(Path('storage/sqlite')))"
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
node --check apps\gui-shell\app.js
```
