# 2026-06-30 - Service Schedule Repository Guard Session

## Objective

Close the remaining service-planning bypass where direct Python repository
calls could insert schedule items without the GUI/CLI business-day block
validation.

## Changes

- Moved service-schedule local date-time and one-business-day validation into
  the SQLite project repository module.
- Reused the repository validation from the GUI/CLI action layer.
- Added a regression test proving direct repository inserts reject multi-day
  business blocks.
- Updated changelog, roadmap, and service-planning documentation.

## Boundaries

- No holiday calendar, capacity planning, Rust service-planning route, or
  release/version bump was added.

## Validation

- Targeted repository and GUI schedule tests: passed with `5` tests.
- `py -m compileall python\emc_locus`: passed.
- `$env:PYTHONPATH='python'; py -m unittest discover -s python\tests`: passed
  with `74` tests.
- `cargo test`: passed with `33` agent tests and `155` core tests.
- `$env:PYTHONPATH='python'; py -m py_compile apps\qt-console\main.py`:
  passed.
- `git diff --check`: passed with expected Windows line-ending warnings only.
