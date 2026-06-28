# Material Planning and Test Taxonomy Session

## Objective

Turn the early console from a light dashboard into a more useful laboratory
surface for metrology records, document evidence, service planning, and
adjustable CEM test categories.

## Changes

- Added SQLite metrology migration version 3 with part numbers, calibration
  periodicity, metrology notes, and `instrument_documents`.
- Added project service planning migration version 2 with schedule rows for
  planned test execution.
- Added test-definition migration version 2 with hierarchical test categories
  seeded for emission/immunity and conducted/radiated CEM branches.
- Extended Python repositories and GUI actions for registering richer
  instruments, attaching equipment documents, calculating next calibration due
  dates, scheduling service items, and creating custom test categories.
- Extended Qt and browser bootstrap tables for material documents, planning,
  and test taxonomy.
- Bumped repository version to `0.2.0`.

## Validation

- Python targeted tests for metrology, GUI actions, test definitions,
  bootstrap, and Qt console: passed.
- Full Python unittest discovery: passed with 33 tests.
- Python compilation checks for repository, GUI action, bootstrap, Qt model, and
  console entry modules: passed.
- Python compileall for `python/emc_locus` and `python/tests`: passed.
- SQLite migration validation: passed with `metrology` at version 3,
  `projects` at version 2, and `test_definitions` at version 2.
- JavaScript syntax check for `apps/gui-shell/app.js`: passed.
- Rust format check: passed.
- Rust test suite: passed with 142 tests.
- Git whitespace check: passed, with Windows CRLF conversion warnings only.

## Next Work

- Replace read-only Qt tables with real operator forms for instrument creation,
  document attachment, schedule editing, and category maintenance.
- Connect the scheduling view to equipment readiness checks before a service
  item can be confirmed.
