# Contract Review Gate Session

## Objective

Prevent projects from entering test planning before their required contract
review evidence is complete, while keeping lighter gates for non-accredited and
investigation work.

## Changes

- Added required contract-review item sets by execution mode.
- Blocked `contract_review -> test_planning` advancement when required items
  are missing.
- Kept accredited projects stricter than non-accredited and investigation
  projects.
- Added tests for accredited blocking/completion and investigation reduced
  gating.
- Bumped repository version to `0.2.4`.

## Validation

- Python targeted GUI action tests: passed with 17 tests.
- Python compilation checks for GUI actions and touched tests: passed.
- Full Python unittest discovery: passed with 40 tests.
- Python compileall for `python/emc_locus` and `python/tests`: passed.
- SQLite migration validation: passed with `metrology` at version 3,
  `projects` at version 2, and `test_definitions` at version 2.
- JavaScript syntax check for `apps/gui-shell/app.js`: passed.
- Rust format check: passed.
- Rust test suite: passed with 142 tests.
- Git whitespace check: passed, with Windows CRLF conversion warnings only.

## Next Work

- Surface missing required contract-review items directly in the Qt project
  detail view.
- Add configurable checklist templates once the lab-specific quality process is
  captured.
