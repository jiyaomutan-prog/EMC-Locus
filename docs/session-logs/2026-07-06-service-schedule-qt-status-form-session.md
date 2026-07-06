# 2026-07-06 - Service Schedule Qt Status Form Session

## Objective

Close the remaining operator-form mismatch where the Qt service-planning status
form still offered the initial `planned` status and terminal planning rows even
though repository transition guards reject unchanged, backward, skipped, and
closed-block status changes.

## Changes

- Limited the Qt status update field to actionable status targets:
  `confirmed`, `in_progress`, `completed`, and `cancelled`.
- Filtered completed and cancelled service-schedule rows out of the Qt status
  update item choices.
- Added Qt form regression coverage for actionable status choices and the
  disabled state when only closed planning rows remain.
- Updated changelog, roadmap, and service-planning documentation.

## Boundaries

- No storage migration, release version bump, tag, Rust API route, or new
  planning workflow was added.
- Repository transition guards remain the authoritative validation boundary;
  the Qt form only avoids presenting targets that are already known to be
  invalid or closed.

## Validation

- Targeted Qt form regression:
  `$env:PYTHONPATH='python'; py -m unittest python.tests.test_qt_console.QtConsoleTests.test_builds_operator_form_specs_for_local_write_repositories`
  - passed with `1` test.
- Qt console regression module:
  `$env:PYTHONPATH='python'; py -m unittest python.tests.test_qt_console`
  - passed with `15` tests.
- Python package compilation:
  `py -m compileall python/emc_locus`
  - passed.
- Rust test suite:
  `cargo test`
  - passed with `46` agent tests and `167` core tests.
- Diff whitespace check:
  `git diff --check`
  - passed; Git reported only Windows CRLF conversion warnings.
