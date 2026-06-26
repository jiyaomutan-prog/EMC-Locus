# Roadmap

## Phase 0: Foundations

Goal: agree on vocabulary, boundaries, and quality principles.

Deliverables:

- product README;
- architecture notes;
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
- first storage schema draft; initial SQLite sketch added.

## Phase 2: Metrology Database

Goal: manage instruments and calibration records.

Deliverables:

- instrument registry;
- calibration status model;
- out-of-service workflow;
- uncertainty references;
- pre-run equipment validity checks.

## Phase 3: Measurement Runtime

Goal: run repeatable test sequences with simulated hardware.

Deliverables:

- simulated instrument driver;
- command and observation log;
- measurement-run model;
- raw dataset checksum;
- data-retention policy hooks.

## Phase 4: Reporting Pipeline

Goal: build a controlled result-to-report path.

Deliverables:

- report package model;
- technical review workflow;
- approval workflow;
- export bundle;
- traceability report for audit.

## Phase 5: Real Instrument Adapters

Goal: connect selected real instruments safely.

Deliverables:

- first transport adapter;
- instrument capability declarations;
- command templates;
- validation against simulated baseline;
- operational safety checklist.

## Near-Term Next Session

The next productive session should start the metrology registry with instrument
identity, calibration validity, and out-of-service status. Keep the same
discipline: domain rule, tests, documentation, changelog, session log, commit.
