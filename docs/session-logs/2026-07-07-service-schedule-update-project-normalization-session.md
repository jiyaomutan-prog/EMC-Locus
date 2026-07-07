# 2026-07-07 - Service Schedule Update Project Normalization Session

## Objective

Close a service-planning consistency gap where list reads could match
constraint-bypassed rows with padded project codes, but direct and audited
status updates still treated the same rows as orphaned.

## Changes

- Matched service-schedule status-update project joins against trimmed
  persisted project codes.
- Returned the canonical project code to audited status updates so project
  audit events are written under the real campaign reference.
- Added regression coverage for direct and audited status updates against
  constraint-bypassed planning rows with padded project references.
- Updated changelog, roadmap, and service-planning documentation.

## Boundaries

- No storage migration, release version bump, tag, Rust API route, or new
  planning workflow was added.
- The guard normalizes lookup and audit context only; it does not rewrite
  imported SQLite rows.

## Validation

- Service-planning repository regression module:
  `$env:PYTHONPATH='python'; py -m unittest python.tests.test_sqlite_repositories.ProjectRepositoryScheduleTests`
  - passed with `44` tests.
- Python package compilation:
  `py -m compileall python/emc_locus`
  - passed.
- Rust test suite:
  `cargo test`
  - passed with `46` agent tests and `167` core tests.
- Whitespace check:
  `git diff --check`
  - passed; Git reported only Windows line-ending normalization warnings.
