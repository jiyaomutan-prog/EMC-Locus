# Changelog

All notable changes to EMC Locus are recorded here.

The project follows Semantic Versioning once public releases begin. During the
foundation phase, `0.x.y` versions may still evolve quickly, but every committed
change should remain traceable through Git history, session logs, and this file.

## [Unreleased]

### Added

- Contract-review stage gate before test planning.
- Authorized contract-review deviation event for incomplete checklists.
- Rust tests for complete checklist, incomplete checklist, authorized deviation,
  project mismatch, and invalid-stage behavior.
- Public BAT-EMC/Nexio feature baseline for product positioning.
- Offline-first architecture direction with split, synchronizable repositories.
- Instrument-control architecture direction for transport-neutral drivers.
- Rust policy models for execution mode, connectivity, repository domains,
  instrument transports, and update policy.
- Public DewesoftX/openDAQ concept baseline for CEM-oriented time-domain
  acquisition and signal processing.
- Signal acquisition architecture for time series, FFT, temporal processing,
  multi-signal math, and synchronized multi-DAQ setups.
- Rust primitives for measurement axes, DAQ interfaces, synchronization methods,
  signal-processing operations, and CEM time-domain test families.
- Consolidated product objectives and non-objectives.
- Rust core crate module split with documented module boundaries.
- Updated recurring development backlog around metrology, simulated DAQ, local
  repository snapshots, and migration work.
- Rust metrology registry primitives for instrument identity, availability,
  calibration records, calibration status, and pre-run readiness checks.
- Tests for accredited, non-accredited, and investigation equipment readiness
  behavior.
- Versioned SQLite migration domains for metrology, projects, test definitions,
  measurement data, and update catalog repositories.
- Python migration validation helper for filename, version, and executable SQL
  checks.
- Rust simulated DAQ source with deterministic inrush time-series fixture.
- Rust signal dataset, sample-rate, channel metadata, processing graph, and
  raw-lineage primitives for FFT/channel-math workflows.
- Rust measurement-run planning model that consumes metrology readiness reports
  and blocks only on blocking pre-run issues.
- Rust field repository package model for offline snapshots, signatures,
  schema-version compatibility, and complete domain coverage.
- Rust simulated instrument runtime with command messages, deterministic
  responses, supported-transport checks, target checks, and ordered observation
  logs.
- Rust dataset evidence model linking accepted measurement-run plans to command
  observations, immutable raw dataset records, file references, and checksums.
- Rust signal execution primitives for deterministic channel arithmetic, peak
  extraction, and DFT magnitude fixtures.
- Rust report package workflow with accredited technical review, approval, and
  issue gates.
- Rust typed instrument setpoints and safety limits, with simulated-runtime
  blocking for commands outside known ranges.
- Python SQLite repository adapters for metrology and project records, backed by
  the versioned migration domains.
- Rust measurement execution session binding accepted plans to simulated runtime
  observations and raw dataset evidence.
- Rust synchronization conflict records for split repository snapshots,
  including conflict status and resolution.
- Rust report export bundle evidence linking issued reports to exported files,
  checksums, and review/approval identities.
- Python SQLite adapter query APIs for instrument, calibration, project, and
  audit-event records.
- Rust signal windowing and deterministic downsampling execution primitives.
- Rust update bundle workflow with signed-package checks, compatibility ranges,
  rollback references, offline-install rules, and live-measurement blocking.
- Rust transport adapter boundary with endpoints, simulated adapter fixture,
  adapter-backed runtime observations, transport mismatch checks, and shared
  safety-limit enforcement.
- Rust synchronization conflict service that turns conflict resolutions into
  audit-required action plans and applies/deferred resolutions safely.
- Python SQLite write APIs for instrument availability/capabilities,
  calibration attachments, project stage changes with audit events, and
  contract-review item completion.

### Planned

- Add optimized FFT and interpolation-based resampling.
- Add update-catalog persistence APIs for signed bundles and install records.
- Add concrete VISA/TCP/IP/serial adapters behind the transport boundary.
- Add sync persistence adapters around conflict action plans.
- Add SQLite adapters for measurement data and test-definition domains.

## [0.1.0] - 2026-06-26

### Added

- Initial product README for EMC Locus.
- Rust workspace with the `emc-locus-core` crate.
- Core project lifecycle model from quotation to archive.
- Project audit trail model with actors, reasons, actions, and ordered events.
- Contract-review checklist model with baseline EN ISO/IEC 17025-oriented items.
- Baseline traceability requirements.
- Python helper package for recurring development session planning.
- Architecture, domain model, EN ISO/IEC 17025 alignment notes, roadmap, and
  storage schema draft.
- Session logs for the initial and autonomous development work.

### Validated

- `py -m compileall python\emc_locus`
- `cargo test`
