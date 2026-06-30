# 2026-06-30 - Python Processing Graph Execution Count Session

## Objective

Close the Python measurement-data traceability gap for processing graph
execution records before adding new runtime surface.

## Changes

- Updated the SQLite measurement-data repository to reject processing graph
  execution records whose `output_artifact_count` does not match the artifacts
  already persisted for the same graph instance.
- Added Python repository tests for completed executions that claim missing
  artifacts or overstate the persisted artifact count.
- Updated the changelog, roadmap, signal-acquisition notes, and storage-schema
  notes with the new invariant.

## Boundaries

- No software version bump was made.
- No database migration, Qt UI, Rust domain change, or new processing operation
  was added.

## Validation

Passed before commit:

- `py -m compileall python\emc_locus`
- `py -m unittest python.tests.test_sqlite_repositories.MeasurementDataRepositoryTests.test_records_revisioned_processing_graph_instances`
- `py -m unittest discover -s python\tests`
- `cargo test` (`23` agent tests and `153` core tests passed)
- `git diff --check`
