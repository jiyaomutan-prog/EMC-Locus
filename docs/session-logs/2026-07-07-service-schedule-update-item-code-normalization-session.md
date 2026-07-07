# 2026-07-07 - Service Schedule Update Item Code Normalization Session

## Objective

Close a service-planning consistency gap where list reads normalized persisted
planning item codes from constraint-bypassed imports, but direct and audited
status updates still searched for the same rows by exact stored `item_code`.

## Changes

- Matched service-schedule status-update lookups against trimmed persisted item
  codes.
- Updated direct and audited status changes by the resolved SQLite row id,
  keeping padded imported keys unchanged while mutating the intended row.
- Wrote audited status-update payloads with the canonical planning item code.
- Rejected ambiguous imported rows when multiple stored item codes normalize to
  the same planning code.
- Added regression coverage for direct updates, audited updates, and ambiguous
  normalized planning codes.
- Updated changelog, roadmap, and service-planning documentation.

## Boundaries

- No storage migration, release version bump, tag, Rust API route, or new
  planning workflow was added.
- The guard normalizes lookup and audit evidence only; it does not rewrite
  imported SQLite `item_code` values.

## Validation

- Service-planning repository regression module:
  `$env:PYTHONPATH='python'; py -m unittest python.tests.test_sqlite_repositories.ProjectRepositoryScheduleTests`
  - passed with `49` tests.
- Python package compilation:
  `py -m compileall python/emc_locus`
  - passed.
- Rust test suite:
  `cargo test`
  - passed with `46` agent tests and `167` core tests.
