# 2026-06-27 Processing Graph Instances Session

## Intent

Make signal-processing graphs persistable as revisioned, auditable instances
linked to source measurement data.

## Changes

- Added Rust processing graph reference and revision value objects.
- Added `ProcessingGraphInstance` with source dataset reference, source dataset
  checksum, graph checksum, creator identity, software version, and raw-lineage
  delegation.
- Rejected empty graph definitions and empty software-version evidence.
- Added a SQLite `processing_graph_instances` migration for revisioned graph
  storage keyed by source dataset, graph reference, and graph revision.
- Added Python repository APIs to add, fetch, and list revisioned processing
  graph instances with source dataset checksum verification.
- Added Rust and Python tests for the new graph-instance behavior.
- Updated changelog, roadmap, product objectives, domain model, storage schema,
  README milestones, and signal analysis notes.

## Validation Evidence

- `cargo fmt --check`: passed.
- `cargo test -q`: 125 Rust tests passed.
- `py -m compileall -q python\emc_locus python\tests`: passed.
- SQLite migration validation: passed with measurement_data=3, metrology=1,
  projects=1, sync=1, test_definitions=1, update_catalog=2.
- Python unittest discovery: 15 tests passed.
- `node --check apps\gui-shell\app.js`: passed.
- `git diff --check`: passed.

## Next Work

- Add richer signal window families.
- Link result artifacts directly to revisioned processing graph instances.
- Expand guarded serial or VISA IO-backed adapters.
