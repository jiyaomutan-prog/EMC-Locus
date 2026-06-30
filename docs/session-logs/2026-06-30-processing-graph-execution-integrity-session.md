# 2026-06-30 - Processing Graph Execution Integrity Session

## Objective

Tighten existing traceability around processing graph execution records before
adding new runtime surface.

## Changes

- Added a domain error for execution artifacts that do not belong to the graph
  reference and revision being recorded.
- Updated `ProcessingGraphExecutionRecord::from_instance` to reject mismatched
  result artifacts before accepting a completed execution record.
- Added Rust coverage for the mismatch path.
- Updated the changelog and roadmap evidence.

## Boundaries

- No software version bump was made.
- No new database migration, Python API, Qt UI, or signal-processing operation
  was added.

## Validation

Passed before commit:

- `py -m compileall python\emc_locus`
- `cargo fmt --check`
- `cargo test` (`23` agent tests and `153` core tests passed)
