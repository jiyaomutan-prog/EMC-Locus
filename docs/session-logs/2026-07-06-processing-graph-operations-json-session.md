# 2026-07-06 - Processing Graph Operations JSON Session

## Objective

Close a measurement-data traceability gap where Python repository callers could
persist legacy or revisioned processing graph records whose `operations_json`
was not a valid structured JSON definition.

## Changes

- Added repository-side validation for processing graph operation definitions.
- Applied the guard to both legacy `processing_graphs` writes and revisioned
  `processing_graph_instances` writes.
- Added regression coverage for invalid JSON and scalar JSON operation
  evidence.
- Updated changelog, roadmap, signal-acquisition notes, and storage-schema
  notes.

## Boundaries

- No storage migration, release version bump, tag, Rust API route, or new
  processing operation was added.
- The guard validates new Python writes only; it does not rewrite historical
  rows that may already exist in local measurement-data databases.

## Validation

- Targeted regression:
  `$env:PYTHONPATH='python'; py -m unittest python.tests.test_sqlite_repositories.MeasurementDataRepositoryTests.test_records_revisioned_processing_graph_instances`
  - passed with `1` test.
- Measurement-data repository regression module:
  `$env:PYTHONPATH='python'; py -m unittest python.tests.test_sqlite_repositories.MeasurementDataRepositoryTests`
  - passed with `4` tests.
- Python package compilation:
  `py -m compileall python/emc_locus`
  - passed.
- Rust test suite:
  `cargo test`
  - passed with `46` agent tests and `167` core tests.
- Diff whitespace check:
  `git diff --check`
  - passed; Git reported only Windows CRLF conversion warnings.
