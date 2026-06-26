# 2026-06-27 Metrology Registry Session

## Intent

Continue autonomous development by addressing the next roadmap dependency:
metrology data needed for EN ISO/IEC 17025-oriented campaign control.

## Changes

- Added a Rust `metrology` module.
- Added metrology dates with calendar validation.
- Added instrument asset codes, instrument families, availability status, and
  calibration requirements.
- Added calibration records with certificate reference, issue date, due date,
  and provider.
- Added an in-memory metrology registry for core-domain behavior.
- Added pre-run equipment readiness reports.
- Connected readiness behavior to execution modes:
  - accredited work blocks on missing or expired required calibration;
  - non-accredited work reports expired calibration without blocking;
  - investigation mode keeps relaxed calibration constraints;
  - out-of-service equipment blocks every mode.
- Updated README, roadmap, changelog, domain model, storage schema draft, and
  core structure notes.

## Validation

- `py -m compileall python\emc_locus`
- `cargo fmt --check`
- `cargo test` passed with 38 tests.
- `git diff --check`

## Next

- Convert the storage schema draft into versioned migrations.
- Add local metrology snapshot planning for offline field work.
- Start a simulated DAQ fixture and signal-processing graph.
