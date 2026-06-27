# 2026-06-27 Processing Graph Artifacts Session

## Intent

Link generated signal-processing outputs to revisioned processing graph
instances instead of treating result files as loose artifacts.

## Changes

- Added Rust `ProcessingGraphResultArtifact`.
- Required result artifacts to come from known graph outputs.
- Restricted graph result artifacts to `processed_signal` and `result_table`
  dataset kinds.
- Preserved graph reference, graph revision, output signal, file reference,
  checksum, and raw-lineage evidence.
- Added a SQLite `processing_graph_instance_artifacts` migration.
- Added Python repository APIs to add and list graph-instance artifacts.
- Added Rust and Python tests for nominal artifact links and rejection paths.
- Updated changelog, roadmap, product objectives, domain model, storage schema,
  README milestones, and signal analysis notes.

## Validation Evidence

- `cargo fmt --check`: passed.
- `cargo test -q`: 127 Rust tests passed.
- `py -m compileall -q python\emc_locus python\tests`: passed.
- SQLite migration validation: passed with measurement_data=4, metrology=1,
  projects=1, sync=1, test_definitions=1, update_catalog=2.
- Python unittest discovery: 15 tests passed.
- `node --check apps\gui-shell\app.js`: passed.
- `git diff --check`: passed.

## Next Work

- Add richer signal window families.
- Add graph-driven execution records for deterministic processing runs.
- Expand guarded serial or VISA IO-backed adapters.
