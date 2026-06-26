# 2026-06-27 Simulated DAQ and Signal Graph Session

## Intent

Make the CEM time-domain direction concrete without depending on real DAQ
hardware or a numeric processing library yet.

## Changes

- Added signal references, signal units, and sample-rate validation.
- Added synchronized signal datasets with DAQ interface and synchronization
  method evidence.
- Added a simulated openDAQ-style source.
- Added a deterministic inrush fixture with voltage and current channels.
- Added signal-processing graph nodes for FFT and channel arithmetic vocabulary.
- Added raw-lineage lookup from derived signal references back to raw acquired
  channels.
- Added tests for invalid references, invalid sample rate, empty datasets,
  deterministic fixture content, unknown graph inputs, duplicate nodes, required
  node inputs, and lineage.
- Updated signal, roadmap, changelog, README, core structure, and backlog docs.

## Validation

- `py -m compileall python\emc_locus`
- `$env:PYTHONPATH='python'; py -c "from pathlib import Path; from emc_locus.migrations import validate_sqlite_migrations; print(validate_sqlite_migrations(Path('storage/sqlite')))"`
- `cargo fmt --check`
- `cargo test` passed with 44 tests.
- `git diff --check`

## Next

- Connect metrology readiness to measurement-run planning.
- Add local snapshot metadata and compatibility checks.
- Start numeric execution for FFT and channel arithmetic.
