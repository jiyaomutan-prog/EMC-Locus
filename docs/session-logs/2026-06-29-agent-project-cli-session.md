# 2026-06-29 - Agent Project CLI Session

## Objective

Make the first project workflow executable through `emc-locus-agent` without
direct Python SQLite writes for this slice.

## Changes

- Added project CLI commands to `emc-locus-agent`:
  - `projects create`;
  - `projects list`;
  - `projects get`;
  - `projects contract-review`;
  - `projects complete-review-item`;
  - `projects to-test-planning`;
  - `projects audit-events`;
  - `sync outbox`.
- Added Rust transaction logic that writes project records and audit events in
  `projects.sqlite` while attaching `sync.sqlite` and inserting a pending
  `sync_operations` row in the same transaction.
- Added deterministic project revisions based on audit sequence numbers.
- Added operation replay handling through `operation_id` to avoid duplicate
  writes.
- Added structured error details for contract-review gate failures.

## Scope Boundaries

- This is still a CLI/local-agent boundary, not the final HTTP loopback API.
- Qt still uses the legacy Python direct-SQLite path for project actions until
  the next migration slice.
- No central synchronization server, PostgreSQL backend, or object storage was
  added.

## Validation Notes

- `cargo fmt --check` passed.
- `cargo clippy --workspace --all-targets -- -D warnings` passed after adding
  the Clippy component to `rust-toolchain.toml` and clearing existing warnings.
- `cargo test --workspace` passed with 12 agent tests and 152 core tests.
- Python compile checks and `py -m unittest discover -s python\tests` passed
  with 49 tests.
- SQLite migration validation passed for `measurement_data`, `metrology`,
  `projects`, `sync`, `test_definitions`, and `update_catalog`.
- `node --check apps\gui-shell\app.js` passed using the bundled Codex Node
  runtime.
- `git diff --check` passed with only Git line-ending conversion warnings.
- A real CLI smoke path passed on a temporary storage root:
  - storage initialization;
  - accredited project creation;
  - rejected transition before contract-review completion;
  - nine review-item completions;
  - accepted transition to `test_planning`;
  - audit inspection;
  - pending outbox inspection;
  - 11 audit events and 11 pending outbox operations observed.
