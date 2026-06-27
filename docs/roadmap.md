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
- SQLite test-definition adapter for standards, test methods, approved
  revisions, processing graph metadata, and evidence steps; initial Python
  adapter added.

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
- traceability report for audit.

## Phase 4b: Signal Acquisition and Analysis

Goal: support CEM tests based on time-domain acquisition and advanced signal
processing, not only level-versus-frequency sweeps.

Deliverables:

- openDAQ-preferred DAQ integration boundary;
- simulated DAQ source;
- synchronized acquisition dataset model; initial Rust model added;
- FFT and temporal-processing pipeline model; initial graph model added;
- numeric DFT/peak/channel arithmetic fixture; initial Rust execution added;
- windowing and deterministic downsampling; initial Rust execution added;
- FFT backend boundary and interpolation resampling; initial Rust execution
  added;
- channel math and signal lineage; initial Rust lineage model added;
- CEM time-domain test families such as railway harmonics, axle counters, and
  inrush measurements.

## Phase 5: Real Instrument Adapters

Goal: connect selected real instruments safely.

Deliverables:

- transport adapter boundary; initial Rust trait and simulated fixture added;
- first concrete hardware transport adapter skeletons; initial Rust VISA,
  TCP/IP, and serial skeletons added;
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
- offline update bundles; initial Rust model added;
- update-catalog persistence APIs; initial Python adapter added;
- rollback and compatibility checks; initial Rust validation added.
- update-install validation evidence mapped into SQLite; initial Python adapter
  added.

## Phase 7: Operator Console

Goal: give the laboratory user a practical, local-first application surface.

Deliverables:

- dashboard for campaign status, readiness, datasets, and update gates; initial
  static GUI shell added;
- project workflow view with selected campaign detail and stage movement;
  initial fixture-driven interaction added;
- metrology, test-definition, measurement-data, and update-management views;
  initial static views added;
- local/offline switch visible in the operator workflow; initial UI control
  added;
- service wiring to the Python repository adapters; initial Python bootstrap
  export and `bootstrap.js` loading path added;
- local write actions and refresh workflow; initial audited project stage action
  added;
- future Rust-backed command execution bridge for instrument runtime actions.

## Near-Term Next Session

The next productive session should add dataset-retention and update-management
local actions behind the GUI shell.

The parallel runtime stream should add IO-backed VISA, TCP/IP, or serial
implementations behind the adapter skeletons.
