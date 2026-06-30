# 2026-06-30 - Python Processing Graph Failed Execution Count Session

## Objective

Lock down Python measurement-data traceability for failed processing graph
executions before adding new runtime surface.

## Changes

- Added Python repository coverage proving failed processing graph executions
  can record zero output artifacts when none are persisted.
- Added Python repository coverage proving failed processing graph executions
  reject an `output_artifact_count` that does not match persisted artifacts.
- Updated changelog, roadmap, signal-acquisition notes, and storage-schema notes
  to make the persisted-artifact count invariant status-independent.

## Boundaries

- No software version bump was made.
- No database migration, Rust domain change, Qt UI change, or new processing
  operation was added.

## Validation

Passed before commit:

- `$env:PYTHONPATH='python'; py -m unittest python.tests.test_sqlite_repositories.MeasurementDataRepositoryTests.test_records_revisioned_processing_graph_instances`
- `py -m compileall python\emc_locus`
- `$env:PYTHONPATH='python'; py -m unittest discover -s python\tests` (`64`
  tests passed)
- `cargo test` (`23` agent tests and `153` core tests passed)
