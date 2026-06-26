# 2026-06-27 Measurement Run Planning Session

## Intent

Connect project execution modes and metrology readiness to the first controlled
measurement-run planning model.

## Changes

- Added a Rust `measurement` module.
- Added measurement-run references and test-method references.
- Added measurement-run plans linking project, method, execution mode,
  equipment, and readiness evidence.
- Added a pre-run gate that rejects empty equipment selections.
- Added a pre-run gate that rejects blocking equipment readiness issues.
- Preserved non-blocking calibration warnings inside accepted non-accredited
  plans.
- Added tests for reference validation, empty equipment, accredited blocking,
  non-accredited warning preservation, and valid accredited planning.
- Updated domain model, roadmap, changelog, README, core structure, and backlog.

## Validation

- `py -m compileall python\emc_locus`
- `$env:PYTHONPATH='python'; py -c "from pathlib import Path; from emc_locus.migrations import validate_sqlite_migrations; print(validate_sqlite_migrations(Path('storage/sqlite')))"`
- `cargo fmt --check`
- `cargo test` passed with 49 tests.
- `git diff --check`

## Next

- Add local repository snapshot metadata and compatibility checks.
- Start a simulated instrument runtime with command and observation logs.
- Connect planned runs to raw dataset records.
