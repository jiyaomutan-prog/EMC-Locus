# 2026-07-01 - Simulated Execution Template Revision Link Session

## Objective

Move the simulated EMC execution/template relationship one step beyond the
approval guard by persisting the approved template revision selected at launch.

## Changes

- Added project migration
  `0005_simulated_execution_template_revision.sql` with nullable selected
  template id, revision id, and definition checksum columns for simulated
  execution attempts.
- Resolved stored template references to the current approved revision before
  simulated execution persistence.
- Returned optional `test_template_revision` evidence in execution detail and
  project execution list DTOs.
- Added audit payload evidence for the selected approved template revision.
- Extended the local API regression for approved-template execution launches.
- Updated changelog, README, roadmap, local-agent, and test-template API notes.

## Boundaries

- Free-form `test_method_reference` values remain supported and do not emit
  `test_template_revision`.
- No variable resolution, copied definition snapshot, execution package,
  campaign instantiation, acquisition, post-processing, or LAB CONSOLE editor
  was added.
- No version bump or release tag was prepared.

## Validation

- `cargo fmt --check`: initially failed on formatting only.
- `cargo fmt`: applied.
- `$env:PYTHONPATH='python'; py -m unittest python.tests.test_release_consistency`: passed.
- `cargo test -p emc-locus-agent local_api_requires_approved_test_template_for_simulated_execution -- --nocapture`: passed.
- `py -m compileall python\emc_locus`: passed.
- `$env:PYTHONPATH='python'; py -c "from pathlib import Path; from emc_locus.migrations import validate_sqlite_migrations; print(validate_sqlite_migrations(Path('storage/sqlite')))"`: passed with `projects: 5`.
- `cargo fmt --check`: passed after formatting.
- `cargo test`: passed with 38 agent tests and 161 core tests.
