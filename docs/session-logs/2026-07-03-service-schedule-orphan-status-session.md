# 2026-07-03 - Service Schedule Orphan Status Session

## Objective

Close a small repository-level status update gap so imported or corrupted
service-schedule rows cannot be mutated when their project reference no longer
resolves.

## Changes

- Updated `ProjectRepository.update_service_schedule_status` to read the target
  planning row together with its project before applying a status change.
- Rejected missing planning rows and orphan planning rows through controlled
  `ValueError` paths before any status mutation.
- Added regression coverage that inserts an orphan planning row through a
  foreign-key-disabled import path and proves the repository refuses the update.
- Documented the guard in the changelog, roadmap, and service-planning notes.

## Boundaries

- No storage migration, release version bump, tag, Rust API route, or new
  planning status vocabulary was added.
- The guard does not restrict normal status updates for valid schedule rows.

## Validation

- Targeted repository schedule tests:
  `$env:PYTHONPATH='python'; py -m unittest python.tests.test_sqlite_repositories.ProjectRepositoryScheduleTests`
  - passed with `19` tests.
- Full Python package compilation:
  `py -m compileall python/emc_locus` - passed.
- Rust test suite:
  `cargo test` - passed with `46` agent tests and `167` core tests.
- Full Python test discovery:
  `$env:PYTHONPATH='python'; py -m unittest discover -s python\tests` -
  passed with `107` tests.
- Whitespace check:
  `git diff --check` - passed with only Git LF-to-CRLF working-copy warnings.
