# 2026-06-30 - Python Processing Graph Software Version Session

## Objective

Close a Python measurement-data traceability gap where processing graph
instances and executions could be persisted with blank software-version
evidence.

## Changes

- Added repository-side non-empty validation for processing graph instance
  `software_version` values.
- Added repository-side non-empty validation for processing graph execution
  `software_version` values.
- Added Python repository coverage for blank software-version rejection on both
  graph instances and graph executions.
- Updated changelog, roadmap, signal-acquisition notes, and storage-schema
  notes with the software-version evidence invariant.

## Boundaries

- No software version bump was made.
- No database migration, Rust domain change, Qt UI change, or new processing
  operation was added.

## Validation

Passed during the session:

- `$env:PYTHONPATH='python'; py -m unittest python.tests.test_sqlite_repositories.MeasurementDataRepositoryTests.test_records_revisioned_processing_graph_instances`
- `py -m compileall python/emc_locus`
- `$env:PYTHONPATH='python'; py -m unittest discover -s python\tests` (`64`
  tests passed)
- `cargo test` (`23` agent tests and `153` core tests passed)
- `git diff --check` (no whitespace errors)
