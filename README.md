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
- React/TypeScript LAB CONSOLE for laboratory management workflows, served by
  the local Rust agent from the versioned production build under `/lab/`.

## Repository Layout

```text
crates/
  emc-locus-agent/       Rust local agent executable skeleton
  emc-locus-core/        Rust domain model and core invariants
apps/
  lab-console/           React/TypeScript LAB CONSOLE workflows
  qt-console/            Qt desktop TEST CONSOLE bootstrap for measurement stations
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
  test-template-api.md Local test-template API contract
  station-setup-api.md Local physical station-setup API contract
  session-logs/          Dated development session records
  competitive-analysis/  Public feature baselines and product positioning
  roadmap.md             Incremental delivery plan
python/
  emc_locus/             Python helper package for planning and automation
storage/
  sqlite/                 Versioned SQLite migrations split by domain
```

## First Useful Milestones

1. Bind a ready physical station setup to a controlled campaign/test execution
   package.
2. Prove correction application and resulting lineage on simulated data before
   introducing real instrument acquisition.

## Development Status

This repository is in active pre-production development. The current focus is
closing complete laboratory workflows across the Rust agent, local storage and
operator interfaces while keeping each domain traceable and offline-capable.

Current software version: `0.20.0`.

Version `0.20.0` adds the first laboratory-wide weekly planning surface. A
laboratory manager can read Monday-to-Friday slots across dossiers, filter them
by operator, location or state, inspect their customer and equipment context,
and move a planned or confirmed slot with an explicit reason. The Rust Local
Agent checks the expected revision and resource availability, then commits the
new reservation, project audit and outbox evidence atomically. A rejected move
names the conflicting dossier and preserves the operator's form values. This
release is a focused weekly coordination workflow, not a capacity engine,
drag-and-drop calendar, absence planner or campaign runtime.

Version `0.19.0` delivers the first usable Locus Lab Management workflow. A
laboratory manager can open an accredited, non-accredited or investigation
dossier, complete the mode-specific review of need, move the dossier to test
planning, reserve an operator and location, understand an overlapping-resource
refusal, confirm the slot and read the resulting history. The Rust Local Agent
is the sole writer for this vertical and commits project data, audit and outbox
evidence atomically. This release plans a first slot inside one dossier; it is
not yet a multi-week resource calendar, campaign runtime or report workflow.

Version `0.18.0` separates a model's correction requirement, its optional
nominal value, the immutable calibration or characterization of one serialized
material, and the reviewed assignment that makes that value usable. The Rust
core now resolves corrections deterministically for a date, execution context,
signal path and conditions such as antenna polarization. LAB CONSOLE starts
from `Matériels réels`: it explains missing evidence, previews a time
conversion or frequency correction, records validity and proof, submits it for
review, activates it explicitly and shows whether the material is ready. A
calibrated value visibly takes precedence over a nominal model value. This
release resolves and previews the selected correction; it still does not apply
it to acquired data.

Version `0.17.0` adds the first revisioned physical measurement-setup workflow
to Locus Test Station. An operator selects real serial-numbered materials,
assigns their roles, connects typed input/output ports, and explicitly pins the
applicable time conversion or frequency response measured by metrology. The
local Rust agent evaluates structural, port, serviceability, calibration,
evidence and nonconformity readiness before the setup can become immutable as
`Prêt à câbler`. The dedicated Qt workflow remains local-first and communicates
only through the agent. It prepares a physical station; it does not acquire a
signal or apply the selected correction to data.

Version `0.16.0` adds the first serial-specific correction workflow to the
physical asset dossier. A metrologist can record either a time-sample
conversion or a frequency response for one inventory/serial number, including
validity, decision, uncertainty, method, certificate reference, and uploaded
content-addressed evidence. The Rust agent validates canonical typed JSON and
persists the immutable event, audit, and outbox atomically. LAB CONSOLE exposes
the workflow from `Matériels réels`, with exact 1440 x 900 and 1280 x 720 visual
acceptance. This release does not yet apply the correction to live acquisition
or choose it automatically during test preparation.

Version `0.15.1` makes the signal-correction workflow readable without the
former measurement-engineering vocabulary. LAB CONSOLE first asks whether
the equipment supplies time samples or a frequency spectrum, then opens the
appropriate conversion or compensation form. Internal identities, checksums,
and storage terms no longer dominate creation, revision, reference, or audit
views. Exact 1440 x 900 and 1280 x 720 Playwright acceptance now guards the
main method and signal-correction paths.
The French domain note
[`docs/domain/signal-corrections.md`](docs/domain/signal-corrections.md)
defines the operator vocabulary and the boundary with serialized metrology
assets.

Version `0.15.0` replaces the ambiguous measurement-engineering vocabulary
with a signal-centered model. Equipment definitions now expose inputs, outputs,
and revision-pinned signal paths. Time-sampled data uses controlled conversions
with gain, offset, and overload/clipping bounds; spectra use frequency responses
with explicit amplitude correction and optional phase correction. LAB CONSOLE
presents these concepts as `Signaux et corrections`, `Conversions temporelles`,
and `Réponses fréquentielles`. This release defines and validates correction
contracts; it does not yet acquire samples, apply corrections at runtime, bind
them directly to a serialized asset, or replace metrology evidence.

Version `0.14.0` gives the Equipment Repository a universal editable `Général`
category inherited by all equipment families, explicit required/optional field
semantics, field editing and archival, and real content-addressed document
upload through the local Rust agent. LAB CONSOLE now also registers a physical,
serial-numbered material in the metrology domain from an approved model. Model
identity and physical asset identity stay separate: manufacturer/model describe
the reusable model, while serial number belongs to the laboratory asset. This
release does not yet implement station wiring, acquisition, FFT, reporting,
RBAC, or central synchronization.

Version `0.13.1` refounds the Equipment Repository user experience around a
laboratory-facing taxonomy and configurable entry templates. A fresh local
database now initializes structural defaults only: seven system root equipment
categories, core subcategories, a minimal equipment-model field dictionary, and
category field rules. It does not create demo equipment models, demo sensors,
demo drivers, or demo acquisition recipes unless an explicit seed command is
run. LAB CONSOLE now exposes Repository Administration for categories, field
definitions, entry-template preview, and a model-creation wizard that starts
from root category and subcategory instead of raw classification enums. Model
definitions preserve the 0.11-0.13 technical core under the hood, with
revisioned `custom_field_values`, template snapshots, audit, outbox evidence,
and hide/show/only demo filtering. This release still does not add physical
asset tracking, station wiring, live hardware drivers, acquisition, FFT,
reporting, RBAC, or central synchronization.

Version `0.13.0` introduced the internal contracts later presented to operators
as signal definitions and corrections. The equipment repository owns
revisioned sensor/transducer definitions, time-domain conversion profiles,
frequency-response definitions, DAQ channel profiles, and logical acquisition
channel recipes. These aggregates are typed in Rust core, persisted in
`equipment.sqlite` with draft/review/approval lifecycle, audit and outbox
evidence, exposed through public local-agent API routes, covered by Python
client helpers, and editable in LAB CONSOLE. Frequency responses support simple
CSV import/export and deterministic 1D evaluation for frequency-dependent
artifacts such as antenna factor, cable loss, amplifier gain, and current
probe transfer. This release still does not perform real DAQ acquisition,
instrument binding, station wiring, FFT, reporting, RBAC, or central sync.

Version `0.12.1` repairs CI/release parity after the 0.12.0 GitHub Actions
run exposed a Windows checkout/build mismatch in the versioned LAB CONSOLE
`dist` bundle. The active validation path now uses npm consistently, the LAB
CONSOLE source and generated text artifacts are pinned to LF line endings, the
CI workflow prints tool versions and uploads failure diagnostics, and
`scripts\validate-ci.ps1` mirrors the GitHub Actions command sequence for
local pre-push validation. No product feature or runtime behavior is added in
this patch release.

Version `0.12.0` productizes equipment physics classification. The equipment
catalog now has backend-owned registries for functional role, signal domain,
port directionality, flow role, and technology tags; classification presets
with port topology; indexed model summaries for catalog filters; an API path to
create draft models from presets; Python client coverage; and LAB CONSOLE
catalog filters plus preset-based creation. It distinguishes ADC converters,
DAQ cards, CAN bus controlled units, RF paths, sensors, sources, actuators,
controllers, software systems, and communication-only ports without treating
metadata as a runtime driver. It is still not a physical fleet deployment
system, certified hardware driver package, acquisition engine, RBAC domain,
or full sensor/DAQ scaling model.

Recommended next vertical: bind a ready station setup to a controlled test
execution package, then apply its pinned correction in a simulated data path
before introducing real acquisition.

Version `0.11.0` delivers the first Equipment Definition Catalog and Driver
Script Studio slice. LAB CONSOLE now has an Equipment space with a functional
model catalog and driver/actions workspace backed by a separate
`equipment.sqlite` domain, typed model and driver definitions in Rust core,
revisioned drafts, approval, CAS saves, audit/outbox evidence, provider status,
structured driver scripts, and deterministic driver simulation. The release
models VISA, CAN, USBTMC, HID, serial, TCP, UDP, manual, and simulation
interfaces honestly: unavailable hardware providers remain reported as not
installed. It is still not a physical fleet redesign, campaign execution
engine, certified hardware driver package, acquisition system, reporting tool,
authentication/RBAC domain, or full sensor/DAQ scaling model.

Version `0.10.0` delivered LAB CONSOLE Template Studio v1. The web surface is a
real React/TypeScript/Vite application served by the Rust local agent under
`/lab/`, with a template library, create/clone flows, structured section
editors, server validation, checksum-based draft saving, submit/approve/derive
workflow, revision history, audit view, system status, demo API seed, launcher
support, unit tests, and Playwright E2E coverage.

Version `0.9.1` is a repair and launchability release. The static LAB CONSOLE
bootstrap remains browser-loadable JavaScript while exposing strict JSON for
the Qt loader, Windows launchers can start the web prototype, the Qt demo, or
the local agent plus Qt from any working directory, and the 0.9.x template
aggregate now uses SQL-level compare-and-swap for draft edits and lifecycle
transitions. Template aggregates expose current approved, latest, and active
draft revisions separately; only one active draft is allowed per template; and
approving a newer revision supersedes older approved revisions with audit and
outbox evidence. Simulated EMC execution attempts also persist the approved
test-template revision selected at launch. This is still not Template Studio,
not a full execution package model, and not a real acquisition runtime.

Version `0.9.0` replaces the provisional test-template model with a real
revisioned aggregate. Test templates now have stable identities, deterministic
content revisions, typed core definitions, canonical JSON and SHA-256
definition checksums, editable drafts with optimistic concurrency, immutable
submitted/approved revisions, derived draft revisions from approved sources,
and audit/outbox evidence tied to explicit revision ids. This is not yet a LAB
CONSOLE editor, a campaign instantiation engine, or a real execution runtime.

Version `0.8.4` adds the first controlled lifecycle transitions for agent-owned
test templates. The local API can submit a draft template for review and approve
an under-review template, with transition rules, idempotent operation replay,
template audit rows, and `test_definitions` outbox operations. This slice still
does not instantiate campaign tests, enforce configurable second approval, or
execute acquisition/post-processing.

Version `0.8.3` adds the first agent-owned test-template draft workflow. The
Rust local agent now initializes `test_definitions.sqlite`, exposes
`/api/v1/test-templates`, stores controlled draft templates with variables,
lock policy, instrumentation chain, sequence, limits, post-processing metadata,
template audit events, and `test_definitions` outbox operations. This slice does
not approve templates, instantiate campaign tests, or execute acquisition.

Version `0.8.2` freezes the GUI and template backbone before another runtime
slice. LAB CONSOLE is now the web-oriented laboratory management surface, TEST
CONSOLE remains the Qt local/offline execution surface, and metrology is treated
as a controlled LAB domain plus a TEST readiness dependency rather than a third
GUI product. The static web shell was refocused on LAB information architecture,
hierarchical navigation, object relationships, and LAB-to-TEST handoff points,
without fake backend writes or execution behavior.

Version `0.8.1` adds the first shared attached-document registry behind the
Rust local agent. LAB CONSOLE, TEST CONSOLE, and controlled domains such as
metrology now have a common metadata shape for controlled documents: owner
domain/entity, classification, storage URI, checksum, revision, applicability,
confidentiality, audit, and sync outbox evidence. This slice does not upload or
store file bytes.

Version `0.8.0` adds the first simulated EMC test execution workflow. A local
operator can launch one simulated EMC attempt through the Rust agent or the
temporary Qt console; the agent runs metrology preflight for the required
instrumentation, persists refused and completed attempts, stores the readiness
verdict and instrumentation snapshot, records a deterministic simulated result
when allowed, and writes project audit plus sync outbox evidence.

Version `0.7.0` consolidates the first agent-backed metrology readiness
vertical slice. A local user can initialize storage, run the Rust agent,
register instruments, record calibration events with certificate manifests,
compute status/readiness, change serviceability, inspect instrument audit and
sync outbox evidence, use the temporary Qt/Python metrology surface through the
agent, and restart the agent while preserving state.

Version `0.6.7` adds the historical migration and E2E confidence layer for the
metrology vertical slice: legacy `calibration_records` are backfilled into
`calibration_events` without losing the original rows, and a real HTTP server
test exercises instrument registration, calibration, readiness, serviceability,
idempotence, restart persistence, audit, and outbox.

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

## Launching LAB CONSOLE And TEST CONSOLE

Windows launchers are available from any working directory:

```powershell
.\scripts\start-lab.ps1
.\scripts\start-lab.ps1 -SeedDemo
.\scripts\start-lab.ps1 -SeedEquipmentDemo
.\scripts\start-lab.ps1 -SeedMeasurementDemo
.\scripts\start-full-demo.ps1
.\scripts\start-qt-demo.ps1 -Mode Static
.\scripts\start-qt-demo.ps1 -Mode Auto
.\scripts\start-agent-qt.ps1
.\scripts\stop-all.ps1
```

Equivalent BAT wrappers are available for shell double-click or `cmd.exe` use:

```bat
scripts\start-lab.bat
scripts\start-full-demo.bat
scripts\start-qt-demo.bat
scripts\start-agent-qt.bat
scripts\stop-all.bat
```

`start-lab` verifies the versioned LAB CONSOLE build, starts or reuses the Rust
agent on `127.0.0.1:8765`, waits for `/api/v1/health` and `/lab/`, then opens
the browser. A normal launch creates no demo equipment records. `-SeedDemo`
creates demonstration templates through the public API. `-SeedEquipmentDemo`
creates explicitly marked demo equipment and driver records. `-SeedMeasurementDemo`
creates approved measurement-engineering demo definitions for a current probe,
biconical antenna, RF cable, RF amplifier, IEPE accelerometer, DAQ analog
input, and a logical `current_A` acquisition recipe. `-Rebuild`
rebuilds the React application when Node/npm is available; normal release launch
uses the committed `apps/lab-console/dist` bundle and does not require Node.
`-StorageRootPath`, `-CargoTargetDirectory` and `-StateName` allow an automated
or parallel session to isolate persistence, build output and process state; the
launcher smoke suite uses these options and never resets the normal
`data/local-agent` root or rebuilds its running executable.
`start-full-demo` opens LAB CONSOLE and then launches TEST CONSOLE Qt against
the same local storage. `start-qt-demo -Mode Static` now uses
`apps/qt-console/demo/bootstrap.json`, a strict JSON fixture owned by the Qt
console, not a LAB web bootstrap script.

Launcher-owned processes are tracked under `logs/launchers/runtime` and can be
stopped without killing unrelated Python or Cargo processes:

```powershell
.\scripts\stop-agent.ps1
.\scripts\stop-all.ps1
```

Revision tracking uses:

- `VERSION` for the current software version;
- `CHANGELOG.md` for user-visible changes;
- `docs/session-logs/` for dated work records;
- Git commits and future signed tags for release evidence;
- `rust-toolchain.toml` and `Cargo.lock` for Rust build reproducibility.

## Validation

Run the CI-equivalent validation before pushing:

```powershell
.\scripts\validate-ci.ps1
```

Useful local shortcuts:

```powershell
.\scripts\validate-ci.ps1 -SkipE2E
.\scripts\validate-ci.ps1 -SkipSmoke
.\scripts\validate-ci.ps1 -NoInstall
```

The script intentionally uses npm for LAB CONSOLE because the repository
commits `package-lock.json` and GitHub Actions runs `npm ci`.

```text
$env:PYTHONPATH='python'; py -m compileall -q python\emc_locus python\tests
$env:PYTHONPATH='python'; py -m py_compile apps\qt-console\main.py
$env:PYTHONPATH='python'; py -m unittest discover -s python\tests
$env:PYTHONPATH='python'; py -c "from pathlib import Path; from emc_locus.migrations import validate_sqlite_migrations; print(validate_sqlite_migrations(Path('storage/sqlite')))"
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cd apps\lab-console
npm ci
npm run typecheck
npm run lint
npm run test
npm run build
npm run test:e2e
cd ..\..
.\scripts\smoke-launchers.ps1 -SkipQtOffscreen
```
