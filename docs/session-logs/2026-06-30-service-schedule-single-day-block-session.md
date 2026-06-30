# 2026-06-30 - Service Schedule Single-Day Block Session

## Objective

Tighten the existing local service-planning validation so each schedule item is
one intra-day business block, not a multi-day reservation hidden inside one row.

## Changes

- Rejected service schedule items whose parsed local start and end datetimes
  fall on different calendar dates.
- Added a regression test for a Thursday-to-Friday business-day block.
- Updated the changelog and roadmap traceability notes.

## Boundaries

- No holiday calendar, resource-capacity model, Rust service-planning route, or
  planning reservation schema was added.

## Validation

- Targeted schedule action tests: passed with `4` tests.
- `py -m compileall python\emc_locus`: passed.
- `cargo test`: passed with `32` agent tests and `155` core tests.
- `$env:PYTHONPATH='python'; py -m unittest discover -s python\tests`: passed
  with `70` tests.
- `git diff --check`: passed with expected Windows line-ending warnings only.
