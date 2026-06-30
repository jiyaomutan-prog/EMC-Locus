# 2026-06-30 - Python Processing Graph Lineage Validation Session

## Objective

Close a Python measurement-data traceability gap for processing graph result
artifacts before adding new runtime surface.

## Changes

- Added repository-side validation for processing graph artifact output signal
  references.
- Added repository-side validation that processing graph artifact
  `raw_lineage_json` is a JSON array of controlled signal references.
- Added Python repository coverage for malformed output signal references,
  non-array raw-lineage JSON, and malformed raw-lineage signal entries.
- Updated changelog, roadmap, signal-acquisition notes, and storage-schema notes
  with the artifact lineage validation invariant.

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
- `git diff --check`
