# 2026-06-29 - Agent Local API Session

## Objective

Expose the validated agent-owned project workflow through a narrow versioned
loopback API.

## Changes

- Added `emc-locus-agent serve`.
- Added a local HTTP dispatcher bound to `127.0.0.1:8765` by default.
- Added routes for:
  - health;
  - project-slice storage initialization;
  - project creation/list/read;
  - contract-review status;
  - contract-review item completion;
  - transition to `test_planning`;
  - project audit events;
  - pending sync outbox.
- Added JSON payload parsing and structured HTTP status mapping for validation,
  conflict, missing-project, and storage-not-initialized errors.
- Added API tests that run the project vertical slice through route handlers.
- Added a real HTTP smoke path using a temporary storage root and loopback port.

## Scope Boundaries

- The API is loopback only by default.
- The API delegates to the same Rust project service path as the CLI.
- Qt is not migrated in this tranche.
- Central synchronization, PostgreSQL, object storage, instrument control, and
  acquisition remain out of scope.

## Validation Notes

- Targeted agent tests passed with 14 tests.
- `cargo fmt --check` passed.
- `cargo clippy --workspace --all-targets -- -D warnings` passed.
- `cargo test --workspace` passed with 14 agent tests and 152 core tests.
- Python compile checks and `py -m unittest discover -s python\tests` passed
  with 49 tests.
- SQLite migration validation passed for `measurement_data`, `metrology`,
  `projects`, `sync`, `test_definitions`, and `update_catalog`.
- Bundled Node syntax check passed for `apps\gui-shell\app.js`.
- Release consistency test passed for version `0.4.5`.
- `git diff --check` passed with only Git line-ending conversion warnings.
- A real HTTP smoke path passed:
  - server startup;
  - `POST /api/v1/storage/initialize`;
  - project creation;
  - rejected early transition with HTTP `409`;
  - nine contract-review item completions;
  - accepted transition to `test_planning`;
  - outbox inspection with 11 pending operations;
  - audit inspection with 11 events.
