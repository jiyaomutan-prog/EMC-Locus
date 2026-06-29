# 2026-06-29 - Python Local Agent Client Session

## Objective

Start migrating the Qt/Python project workflow away from direct SQLite writes by
adding a thin client for the local Rust agent.

## Changes

- Added `python/emc_locus/local_agent_client.py`.
- Added structured `LocalAgentError` handling for agent JSON errors.
- Added UUID-backed operation-id generation for project write retries.
- Added optional `agent_url` paths to:
  - `create_project_record`;
  - `complete_contract_review_item_action`;
  - `advance_project_stage`.
- Added `--agent-url`, `--operation-id`, `--correlation-id`, and `--device-id`
  to the relevant Python CLI actions.
- Added `--agent-url` to the Qt console and routed project creation and
  contract-review item forms through the agent when configured.
- Documented the agent-backed and legacy-direct-SQLite split in the Qt README.

## Scope Boundaries

- The legacy direct SQLite path remains the default when `agent_url` is not
  provided.
- Qt still reads current table data from repository paths in this tranche.
- Metrology, service scheduling, test categories, measurement data, updates, and
  runtime actions remain legacy direct SQLite.

## Validation Notes

- Targeted local-agent client tests passed with 6 tests.
- Full Python unittest discovery passed with 55 tests.
- A live Python-client smoke path passed against the real Rust loopback server:
  - project creation returned `contract_review`;
  - premature transition returned HTTP `409`;
  - nine review items were completed;
  - transition returned `test_planning`.
- `cargo fmt --check` passed.
- `cargo clippy --workspace --all-targets -- -D warnings` passed.
- `cargo test --workspace` passed with 14 agent tests and 152 core tests.
- Python compile checks and Qt console `py_compile` passed.
- SQLite migration validation passed.
- Bundled Node syntax check passed.
