# Roadmap

## Phase 0: Foundations

Goal: agree on vocabulary, boundaries, and quality principles.

Deliverables:

- product README;
- consolidated product objectives;
- architecture notes;
- core crate structure notes;
- domain model;
- EN ISO/IEC 17025 alignment notes;
- minimal Rust core crate;
- minimal Python helper package.

## Phase 1: Campaign Core

Goal: model the complete project lifecycle.

Deliverables:

- project and campaign entities;
- lifecycle state machine; initial Rust model added;
- audit events; initial Rust project audit log added;
- contract-review checklist; initial Rust checklist added;
- contract-review stage gate; initial Rust gate added;
- SQLite project stage/audit and contract-review write APIs; initial Python
  updates added;
- public BAT-EMC feature baseline; initial analysis added;
- quality modes and offline policy primitives; initial Rust model added;
- DewesoftX/openDAQ concept baseline; initial analysis added;
- signal acquisition and analysis primitives; initial Rust model added;
- first storage schema draft; initial SQLite sketch added.
- versioned split SQLite migrations; initial domain migrations added.
- Rust core module split; initial structure added.
- Rust application-service boundary; initial project stage advancement service
  added as first migration path away from direct Python write rules.
- SQLite test-definition adapter for standards, test methods, approved
  revisions, processing graph metadata, and evidence steps; initial Python
  adapter added.
- Adjustable test-category taxonomy seeded with emission/immunity and
  conducted/radiated branches; initial SQLite/Python/Qt/browser support added.

## Phase 2: Metrology Database

Goal: manage instruments and calibration records.

Deliverables:

- instrument registry; initial Rust model added;
- calibration status model; initial Rust model added;
- out-of-service workflow; initial Rust blocking rule added;
- uncertainty references;
- pre-run equipment validity checks; initial Rust readiness report added.
- SQLite metrology adapter; initial Python smoke adapter added.
- SQLite metrology query APIs; initial Python lookups added.
- SQLite metrology write APIs; initial Python updates added.
- Revisioned instrument category taxonomy for electronics, EMC, thermal,
  acoustic, shock/vibration, radio/RF, and data-monitoring equipment; initial
  SQLite/Python/Qt support added.
- Instrument registration fields for manufacturer, model, serial, part number,
  calibration periodicity, calculated next calibration, notes, and attached
  certificates, datasheets, transducer sheets, and scripts; initial
  SQLite/Python/Qt/browser support added.
- Agent-backed metrology readiness vertical slice; instrument registration,
  calibration events with certificate manifests, computed status, serviceability
  changes, readiness assessment, audit/outbox evidence, Qt/Python agent path,
  historical calibration migration, and real HTTP restart E2E coverage delivered
  as the `0.7.0` baseline.
- First simulated EMC execution workflow delivered as the `0.8.0` baseline:
  operator launch attempt, test-context metrology preflight, structured refusal,
  persisted completed result, instrumentation snapshot, project audit/outbox,
  local API routes, and a minimal Qt operator form.

## Phase 3: Measurement Runtime

Goal: run repeatable test sequences with simulated hardware.

Deliverables:

- measurement-run planning model; initial Rust pre-run gate added;
- simulated instrument driver; initial Rust runtime added;
- command and observation log; initial Rust observation log added;
- measurement-run execution model; initial Rust execution binding added;
- raw dataset checksum; initial Rust dataset evidence model added;
- SQLite measurement-data adapter; initial Python adapter added;
- data-retention policy hooks; initial Rust workflow, SQLite evidence, and
  Python adapter APIs added.

## Phase 4: Reporting Pipeline

Goal: build a controlled result-to-report path.

Deliverables:

- report package model; initial Rust model added;
- technical review workflow; initial Rust gate added;
- approval workflow; initial Rust gate added;
- export bundle; initial Rust evidence model added;
- traceability report for audit; initial Rust view added;
- exchange-attempt review summaries; total and maximum attempts per run added.

## Phase 4b: Signal Acquisition and Analysis

Goal: support CEM tests based on time-domain acquisition and advanced signal
processing, not only level-versus-frequency sweeps.

Deliverables:

- openDAQ-preferred DAQ integration boundary;
- simulated DAQ source;
- synchronized acquisition dataset model; initial Rust model added;
- FFT and temporal-processing pipeline model; initial graph model added;
- numeric DFT/peak/channel arithmetic fixture; initial Rust execution added;
- windowing and deterministic downsampling; initial Rust execution added, with
  rectangular, Hann, Hamming, Blackman, and flat-top windows;
- FFT backend boundary and interpolation resampling; initial Rust execution
  added;
- optimized radix-2 FFT-compatible backend with DFT fallback; initial Rust
  execution added;
- windowed FFT execution with retained window metadata and optimized-backend
  compatibility; initial Rust execution added;
- channel math and signal lineage; initial Rust lineage model added;
- persisted processing graph instances; initial Rust instance model, SQLite
  migration, and Python repository APIs added;
- result artifacts linked to revisioned processing graph instances; initial
  Rust artifact model, SQLite migration, and Python repository APIs added;
- graph-driven execution records; initial Rust model, SQLite migration, and
  Python repository APIs added;
- execution records now reject artifacts from a different processing graph
  reference or revision, preserving graph-to-result evidence integrity;
- Python measurement-data writes now reject graph execution records whose
  output artifact count does not match the artifacts persisted for that graph
  instance;
- failed processing graph executions are covered by the same persisted-artifact
  count invariant as completed executions;
- Python measurement-data writes now reject processing graph result artifacts
  whose output signal reference or raw-lineage JSON cannot be parsed as
  controlled signal traceability evidence;
- Python measurement-data writes now reject processing graph instances and
  executions whose software-version evidence is blank;
- CEM time-domain test families such as railway harmonics, axle counters, and
  inrush measurements.

## Phase 5: Real Instrument Adapters

Goal: connect selected real instruments safely.

Deliverables:

- transport adapter boundary; initial Rust trait and simulated fixture added;
- first concrete hardware transport adapter skeletons; initial Rust VISA,
  TCP/IP, and serial skeletons added;
- TCP/IP IO-backed exchange; initial Rust standard-library implementation and
  local socket test added;
- transport attempt traceability; adapter-backed observations now retain
  exchange counts, including TCP/IP retry attempts;
- structured serial endpoint settings; initial Rust parser and adapter
  validation added;
- structured VISA resource settings; initial Rust parser and adapter validation
  added;
- VISA TCP/IP resources exchanged through the guarded TCP socket path; initial
  Rust local socket coverage added;
- instrument capability declarations;
- command templates;
- validation against simulated baseline;
- operational safety checklist; initial typed safety-limit model added.

## Phase 6: Offline, Sync, and Updates

Goal: make EMC Locus robust outside the laboratory network.

Deliverables:

- split local repositories;
- signed reference snapshots; initial Rust field-package model added;
- snapshot schema compatibility checks; initial Rust validation added;
- synchronization conflict workflow; initial Rust action-plan service added;
- synchronization conflict records; initial Rust model added;
- synchronization conflict persistence; initial SQLite migration and Python
  adapter added;
- local agent storage initialization/status/verification for the project
  vertical slice; initial Rust CLI support added for project and sync databases;
- project/sync multi-SQLite atomicity policy; rollback journal enforcement and
  status reporting added for the agent-owned vertical slice;
- local agent project write path; initial Rust CLI support added for project
  creation, contract-review completion, transition to test planning, audit
  inspection, and pending outbox inspection;
- local agent versioned loopback API; initial Rust HTTP routes added for the
  project vertical slice and pending outbox inspection;
- Python/Qt project client path; initial thin Python client added so project
  creation, contract-review completion, and planning transition can call the
  local agent when configured;
- Qt/Python project read path; project list, contract-review status, audit
  events, and sync outbox now read through the local agent when configured;
- hardened 0.6.0 project vertical slice; idempotence, DTOs, module split,
  SQLite atomicity, Qt/Python agent reads, and CI consolidated into the release
  baseline;
- real local-agent E2E coverage; initial automated HTTP server test added for
  the project vertical slice, including restart and persistence verification;
- Qt project transition form; initial operator form added for the agent-backed
  transition to `test_planning`;
- offline update bundles; initial Rust model added;
- update-catalog persistence APIs; initial Python adapter added;
- rollback and compatibility checks; initial Rust validation added.
- update-install validation evidence mapped into SQLite; initial Python adapter
  added.

## Phase 7: Operator Console

Goal: give the laboratory user a practical, local-first application surface.

Deliverables:

- dashboard for campaign status, readiness, datasets, and update gates; initial
  static workflow prototype added;
- Qt desktop console for local measurement stations; initial PySide6 bootstrap
  added;
- project workflow view with selected campaign detail and stage movement;
  initial fixture-driven interaction added;
- metrology, test-definition, measurement-data, and update-management views;
  initial static prototype views added;
- local/offline switch visible in the operator workflow; initial UI control
  added;
- service wiring to the Python repository adapters; initial Python bootstrap
  export and `bootstrap.js` loading path added;
- local write actions and refresh workflow; initial audited project stage action
  and dataset-retention and update-management actions added;
- service planning table for scheduling test execution by project, operator,
  category, location, EUT, status, and planned time window; initial
  SQLite/Python/Qt/browser support added;
- local service-planning actions now reject malformed local date-time blocks and
  weekend-crossing schedule items before repository writes;
- local service-planning actions now also reject multi-day business-day blocks,
  keeping each scheduled execution item as a single intra-day lab block;
- future Qt model/view screens backed by application services;
- future Rust-backed command execution bridge for instrument runtime actions.

## Near-Term Next Session

The next productive session should deepen the simulated EMC execution workflow:
bind it to approved test definitions, persist method parameters, and create
measurement-data evidence records for the simulated result.

The parallel runtime stream should continue guarded serial or VISA IO behind the
adapter skeletons.

The UI stream should keep migrating Qt actions toward Rust-owned application
services instead of direct SQLite writes.
