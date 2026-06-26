# 2026-06-27 Reporting Gates Session

## Intent

Add the first controlled result-to-report workflow for accredited and
non-accredited execution modes.

## Changes

- Added a Rust `reporting` module.
- Added report numbers and revisions.
- Added report package status workflow.
- Added technical-review submission.
- Added completed technical review with reviewer identity.
- Added approval with approver identity.
- Added issue gate.
- Enforced technical review before approval for accredited work.
- Enforced approval before issue for accredited work.
- Allowed non-accredited reports to be issued without the same formal approval
  gate.
- Added tests for empty report identifiers, accredited review/approval gates,
  valid accredited issue flow, non-accredited issue flow, and invalid
  transitions.
- Updated domain model, roadmap, changelog, README, core structure, and backlog.

## Validation

- `py -m compileall python\emc_locus`
- `$env:PYTHONPATH='python'; py -c "from pathlib import Path; from emc_locus.migrations import validate_sqlite_migrations; print(validate_sqlite_migrations(Path('storage/sqlite')))"`
- `cargo fmt --check`
- `cargo test` passed with 76 tests.
- `git diff --check`

## Next

- Add persistent adapters for metrology and project repositories.
- Add typed safety limits for instrument commands.
- Add report export bundle evidence.
