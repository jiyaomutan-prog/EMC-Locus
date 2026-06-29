# EMC Locus 0.5.0 - Agent-Backed Project Vertical Slice

## Delivered

Version `0.5.0` makes the first complete project workflow executable through
`emc-locus-agent`:

1. initialize local `projects.sqlite` and `sync.sqlite`;
2. start the local loopback API;
3. create an accredited project;
4. read the project;
5. reject premature transition to `test_planning`;
6. complete required contract-review items;
7. accept transition to `test_planning`;
8. inspect project audit events;
9. inspect pending sync outbox operations;
10. replay an operation idempotently without duplicating audit/outbox rows;
11. verify relaxed gates for `non_accredited` and `investigation` modes;
12. restart the server and verify persisted project state;
13. show local-agent state in the Qt console header;
14. submit Qt project forms through a worker instead of the main UI thread.

## Main Commands

```text
cargo run -q -p emc-locus-agent -- storage init --storage-root data\agent --migrations-root storage\sqlite
cargo run -q -p emc-locus-agent -- serve --storage-root data\agent --migrations-root storage\sqlite --bind 127.0.0.1:8765
py apps\qt-console\main.py --projects-db data\agent\projects.sqlite --agent-url http://127.0.0.1:8765
```

## Agent-Backed

- project creation;
- project list/read through API;
- contract-review status;
- contract-review item completion;
- transition to `test_planning`;
- project audit event inspection;
- pending sync outbox inspection.
- storage status through `GET /api/v1/storage/status`;
- Qt project forms with visible local-agent status.

## Legacy Direct SQLite

- metrology entry and documents;
- service scheduling;
- test-category maintenance;
- measurement data;
- update actions;
- instrument runtime and acquisition actions.

## Validation

The release was validated with:

```text
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
$env:PYTHONPATH='python'; py -m compileall -q python\emc_locus python\tests
$env:PYTHONPATH='python'; py -m py_compile apps\qt-console\main.py
$env:PYTHONPATH='python'; py -m unittest discover -s python\tests
$env:PYTHONPATH='python'; py -c "from pathlib import Path; from emc_locus.migrations import validate_sqlite_migrations; print(validate_sqlite_migrations(Path('storage/sqlite')))"
node --check apps\gui-shell\app.js
git diff --check
```

The Rust test suite includes a real HTTP server E2E test that verifies
operation replay and restart/persistence for the project vertical slice.
The final release validation passed with 19 agent tests, 152 core tests, 59
Python tests, all SQLite migration domains valid, bundled Node syntax check
valid, and `git diff --check` clean apart from Windows CRLF notices.

## Remaining Work

- Replace remaining Qt project reads with API-backed reads where practical.
- Continue migrating metrology, planning, test definitions, and update actions
  behind the agent in separate slices.
