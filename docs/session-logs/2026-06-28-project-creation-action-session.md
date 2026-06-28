# Project Creation Action Session

## Objective

Remove a practical blocker in the planning workflow: a service item should be
able to start from a project created locally in EMC Locus, not only from
pre-existing fixture or imported rows.

## Changes

- Added an audited project creation repository operation that inserts the
  project and first `project_created` audit event in one transaction.
- Added a `create-project` GUI action and CLI command with bootstrap refresh
  support.
- Added a `Nouveau projet` Qt form that is enabled whenever a local projects
  repository path is supplied.
- Wired the Qt form bridge to the project creation action.
- Bumped repository version to `0.2.2`.

## Validation

- Python targeted tests for Qt forms and GUI actions: passed with 22 tests.
- Python compilation checks for repositories, GUI actions, exports, Qt console,
  Qt models, and touched tests: passed.
- Full Python unittest discovery: passed with 37 tests.
- Python compileall for `python/emc_locus` and `python/tests`: passed.
- SQLite migration validation: passed with `metrology` at version 3,
  `projects` at version 2, and `test_definitions` at version 2.
- JavaScript syntax check for `apps/gui-shell/app.js`: passed.
- Rust format check: passed.
- Rust test suite: passed with 142 tests.
- Git whitespace check: passed, with Windows CRLF conversion warnings only.

## Next Work

- Add contract-review item forms so a newly created accredited project can move
  toward planning with visible checklist evidence.
- Add schedule readiness checks that combine project stage, test category, and
  metrology availability.
