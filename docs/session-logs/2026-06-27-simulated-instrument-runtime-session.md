# 2026-06-27 Simulated Instrument Runtime Session

## Intent

Start a serious instrument-control foundation with deterministic simulated
commands and observation logs before introducing real communication adapters.

## Changes

- Added stable transport slugs for logs and adapters.
- Added instrument command messages.
- Added instrument responses.
- Added instrument commands with target instrument and requested transport.
- Added instrument observations with sequence, command, response, and success.
- Added a simulated instrument runtime.
- Added validation for command target mismatch and unsupported transport.
- Added deterministic simulated responses for query and set commands.
- Added tests for message validation, ordered observations, wrong target,
  unsupported transport, and transport slugs.
- Updated instrument-control architecture, roadmap, changelog, README, core
  structure, and backlog.

## Validation

- `py -m compileall python\emc_locus`
- `$env:PYTHONPATH='python'; py -c "from pathlib import Path; from emc_locus.migrations import validate_sqlite_migrations; print(validate_sqlite_migrations(Path('storage/sqlite')))"`
- `cargo fmt --check`
- `cargo test` passed with 61 tests.
- `git diff --check`

## Next

- Connect accepted measurement-run plans to raw dataset records and checksums.
- Start numeric signal-processing execution fixtures.
- Add typed safety limits for instrument commands.
