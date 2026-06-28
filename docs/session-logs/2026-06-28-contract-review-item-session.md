# Contract Review Item Session

## Objective

Make a newly created project more useful by allowing operators to complete
contract-review checklist items locally with audit evidence and visible
bootstrap output.

## Changes

- Added atomic completion of a contract-review item with an audit event.
- Added a `complete-contract-review-item` GUI action and CLI command.
- Added contract-review item rows to the GUI bootstrap contract.
- Added Qt and browser tables for contract-review items.
- Added a Qt `Revue contrat` form for completing checklist items.
- Bumped repository version to `0.2.3`.

## Validation

- Python targeted tests for Qt forms, GUI bootstrap, and GUI actions: passed
  with 24 tests.
- Python compilation checks for repositories, GUI actions, bootstrap, exports,
  Qt console, Qt models, and touched tests: passed.
- Full Python unittest discovery: passed with 38 tests.
- Python compileall for `python/emc_locus` and `python/tests`: passed.
- SQLite migration validation: passed with `metrology` at version 3,
  `projects` at version 2, and `test_definitions` at version 2.
- JavaScript syntax check for `apps/gui-shell/app.js`: passed.
- Rust format check: passed.
- Rust test suite: passed with 142 tests.
- Git whitespace check: passed, with Windows CRLF conversion warnings only.

## Next Work

- Add predefined EN ISO/IEC 17025-oriented checklist templates per execution
  mode.
- Gate project advancement from contract review to planning on required
  checklist completion, with relaxations for non-accredited and investigation
  modes.
