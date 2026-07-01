# 2026-07-01 - Service Schedule Canonical Datetime Session

## Objective

Close a remaining service-planning ambiguity where Python accepted non-canonical
ISO datetime variants for schedule rows, even though operators and docs use
`YYYY-MM-DDTHH:MM` local timestamps.

## Changes

- Tightened the shared service-schedule datetime parser to require
  `YYYY-MM-DDTHH:MM` before parsing.
- Kept the existing one-intra-day and weekday-only planning block rules.
- Added a repository-level regression test so direct Python repository callers
  cannot insert week-date or compact datetime variants.
- Updated changelog, roadmap, and service-planning documentation.

## Boundaries

- No holiday calendar, capacity planning, Rust service-planning route, or
  release/version bump was added.

## Validation

- Targeted repository schedule tests:
  `$env:PYTHONPATH='python'; py -m unittest python.tests.test_sqlite_repositories.ProjectRepositoryScheduleTests`
  - passed with `2` tests.
- `py -m compileall python\emc_locus` - passed.
- `cargo test` - passed with `44` agent tests and `162` core tests.
- `$env:PYTHONPATH='python'; py -m unittest discover -s python\tests` -
  passed with `85` tests.
