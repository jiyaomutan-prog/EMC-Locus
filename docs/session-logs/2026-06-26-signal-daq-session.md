# Signal DAQ Session - 2026-06-26

## Objective

Capture a missing CEM capability area: tests that rely on time-domain DAQ
signals, FFT or temporal processing, mathematical operations between signals,
and advanced synchronization across multiple DAQ devices.

## Changes

- Added a DewesoftX/openDAQ public concept baseline.
- Added a signal acquisition and analysis architecture section.
- Added roadmap entries for time-domain DAQ and signal processing.
- Added Rust primitives for:
  - measurement axes;
  - DAQ interfaces, with openDAQ as the preferred generic interface;
  - signal source kinds;
  - synchronization methods;
  - signal-processing operations;
  - CEM time-domain test families;
  - a default CEM time-domain workflow profile.
- Added Rust tests for openDAQ preference, mixed time/frequency processing,
  multi-DAQ synchronization methods, processing operations, and CEM
  time-domain test families.
- Updated `CHANGELOG.md`, `README.md`, `docs/architecture.md`, and
  `docs/roadmap.md`.

## Product Rationale

EMC Locus must not be limited to BAT-EMC-style level-versus-frequency sweeps.
CEM also includes railway harmonics, axle-counter measurements, inrush current,
transient capture, and investigation workflows that need time signals, FFT,
filters, event timing, and math between channels.

## Public Research Notes

The DewesoftX and openDAQ notes use public product, documentation, and GitHub
pages only. EMC Locus should learn from the public concepts while remaining an
original CEM-focused implementation.

## Validation Notes

Run before commit:

```text
py -m compileall python\emc_locus
cargo fmt --check
cargo test
```

Result: Python compilation passed, `cargo fmt --check` passed, and 29 Rust tests passed.

## Next Recommended Step

Create a simulated DAQ source and a minimal signal-processing graph fixture:
time-series input, FFT output, channel arithmetic, event timing, and result
lineage back to raw data.
