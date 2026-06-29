# 2026-06-29 Service Schedule Business Days Session

## Objective

Tighten local service-planning validation so operator schedule items are checked
as real local date-time blocks and cannot be created across non-working weekend
days.

## Changes

- Added parsed ISO local date-time validation to the Python GUI/CLI scheduling
  action.
- Replaced string ordering for planned start/end values with `datetime`
  comparison.
- Rejected timezone-bearing schedule inputs and schedule blocks that include
  Saturday or Sunday.
- Added regression tests for accepted weekday planning, weekend rejection, and
  malformed local date-time rejection.
- Updated the changelog and roadmap traceability notes.

## Validation

Initial targeted validation:

```text
$env:PYTHONPATH='python'; py -m unittest python.tests.test_sqlite_repositories.GuiActionTests.test_schedule_service_item_records_planned_test_and_refreshes_bootstrap python.tests.test_sqlite_repositories.GuiActionTests.test_schedule_service_item_rejects_weekend_blocks python.tests.test_sqlite_repositories.GuiActionTests.test_schedule_service_item_rejects_invalid_local_datetime
  -> 3 tests OK
```

Final validation for this session:

```text
$env:PYTHONPATH='python'; python -m unittest python.tests.test_sqlite_repositories.GuiActionTests.test_schedule_service_item_records_planned_test_and_refreshes_bootstrap python.tests.test_sqlite_repositories.GuiActionTests.test_schedule_service_item_rejects_weekend_blocks python.tests.test_sqlite_repositories.GuiActionTests.test_schedule_service_item_rejects_invalid_local_datetime
  -> 3 tests OK
cargo fmt --check
cargo test --workspace
  -> 4 agent tests OK and 152 core tests OK
$env:PYTHONPATH='python'; python -m compileall -q python\emc_locus python\tests
$env:PYTHONPATH='python'; python -m py_compile apps\qt-console\main.py
$env:PYTHONPATH='python'; python -m unittest discover -s python\tests
  -> 48 tests OK
$env:PYTHONPATH='python'; python -c "from pathlib import Path; from emc_locus.migrations import validate_sqlite_migrations; print(validate_sqlite_migrations(Path('storage/sqlite')))"
  -> {'measurement_data': 7, 'metrology': 3, 'projects': 2, 'sync': 3, 'test_definitions': 2, 'update_catalog': 2}
C:\Users\gtrai\.cache\codex-runtimes\codex-primary-runtime\dependencies\node\bin\node.exe --check apps\gui-shell\app.js
cargo run -q -p emc-locus-agent -- health --storage-root storage
  -> JSON health report OK
git diff --check
  -> OK, with expected Windows LF/CRLF warnings only
cargo clippy --workspace --all-targets -- -D warnings
  -> not run: cargo-clippy is not installed for toolchain 1.96.0-x86_64-pc-windows-msvc
```

## Next Work

- Consider configurable laboratory calendars and holidays once the planning
  workflow has a dedicated Rust application-service boundary.
- Continue the near-term runtime stream around guarded serial/VISA adapter
  implementation.
