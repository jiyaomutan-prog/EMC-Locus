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

### Planned

- Start the metrology registry with instruments and calibration records.
- Introduce persistent migrations from the storage schema draft.
- Connect execution modes to stage gates and report approval rules.
- Implement a simulated DAQ source and signal-processing pipeline fixtures.

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
