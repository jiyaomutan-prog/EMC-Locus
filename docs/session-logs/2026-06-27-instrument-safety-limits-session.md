# 2026-06-27 Instrument Safety Limits Session

## Intent

Add typed safety checks before introducing any real instrument transport adapter.

## Changes

- Added instrument quantities for frequency, level, voltage, and current.
- Added typed instrument setpoints.
- Added typed instrument safety limits.
- Added validation for inverted safety ranges.
- Extended instrument commands with optional setpoints.
- Extended the simulated runtime with safety limits.
- Blocked commands whose typed setpoint is outside a known limit.
- Ensured blocked commands do not create observations.
- Added tests for invalid limits, accepted in-range commands, and blocked
  out-of-range commands.
- Updated instrument-control architecture, roadmap, changelog, README, and
  backlog.

## Validation

- `py -m compileall python\emc_locus`
- `$env:PYTHONPATH='python'; py -c "from pathlib import Path; from emc_locus.migrations import validate_sqlite_migrations; print(validate_sqlite_migrations(Path('storage/sqlite')))"`
- `cargo fmt --check`
- `cargo test` passed with 79 tests.
- `git diff --check`

## Next

- Add persistent adapters for metrology and project repositories.
- Connect measurement-run execution to simulated runtime and dataset evidence.
- Add optimized FFT/windowing and resampling execution.
