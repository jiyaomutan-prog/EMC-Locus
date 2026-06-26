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
- public BAT-EMC feature baseline; initial analysis added;
- quality modes and offline policy primitives; initial Rust model added;
- DewesoftX/openDAQ concept baseline; initial analysis added;
- signal acquisition and analysis primitives; initial Rust model added;
- first storage schema draft; initial SQLite sketch added.
- versioned split SQLite migrations; initial domain migrations added.
- Rust core module split; initial structure added.

## Phase 2: Metrology Database

Goal: manage instruments and calibration records.

Deliverables:

- instrument registry; initial Rust model added;
- calibration status model; initial Rust model added;
- out-of-service workflow; initial Rust blocking rule added;
- uncertainty references;
- pre-run equipment validity checks; initial Rust readiness report added.

## Phase 3: Measurement Runtime

Goal: run repeatable test sequences with simulated hardware.

Deliverables:

- measurement-run planning model; initial Rust pre-run gate added;
- simulated instrument driver; initial Rust runtime added;
- command and observation log; initial Rust observation log added;
- measurement-run execution model;
- raw dataset checksum; initial Rust dataset evidence model added;
- data-retention policy hooks.

## Phase 4: Reporting Pipeline

Goal: build a controlled result-to-report path.

Deliverables:

- report package model;
- technical review workflow;
- approval workflow;
- export bundle;
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
- channel math and signal lineage; initial Rust lineage model added;
- CEM time-domain test families such as railway harmonics, axle counters, and
  inrush measurements.

## Phase 5: Real Instrument Adapters

Goal: connect selected real instruments safely.

Deliverables:

- first transport adapter;
- instrument capability declarations;
- command templates;
- validation against simulated baseline;
- operational safety checklist.

## Phase 6: Offline, Sync, and Updates

Goal: make EMC Locus robust outside the laboratory network.

Deliverables:

- split local repositories;
- signed reference snapshots; initial Rust field-package model added;
- snapshot schema compatibility checks; initial Rust validation added;
- synchronization conflict workflow;
- offline update bundles;
- rollback and compatibility checks.

## Near-Term Next Session

The next productive session should add report approval gates for accredited
workflows.

The parallel storage stream should add persistent adapters for metrology and
project repositories.
