# 2026-07-08 - Service Schedule Optional Text Insert Session

## Objective

Close a service-planning validation gap where direct Python callers could pass
non-text optional `test_category_code`, `test_method_code`, or `notes` values
and persist corrupted optional traceability fields, including through the
audited planning insert path.

## Changes

- Made optional text normalization reject non-text values when a value is
  provided.
- Passed explicit field names through service-planning repository and GUI/CLI
  action calls so validation errors remain controlled and field-specific.
- Added regression coverage proving direct and audited service-schedule inserts
  reject non-text optional values before writing planning rows or audit events.
- Updated changelog, roadmap, and service-planning documentation.

## Boundaries

- No storage migration, release version bump, tag, Rust API route, or new
  planning workflow was added.
- The guard rejects new malformed input; it does not rewrite existing SQLite
  planning rows.

## Validation

- Targeted optional-text insert regressions:
  `$env:PYTHONPATH='python'; py -m unittest python.tests.test_sqlite_repositories.ProjectRepositoryScheduleTests.test_repository_rejects_non_text_service_schedule_optional_fields_on_insert python.tests.test_sqlite_repositories.ProjectRepositoryScheduleTests.test_repository_rejects_non_text_service_schedule_notes_on_audit_insert`
  - passed.
- Service-planning repository regression module:
  `$env:PYTHONPATH='python'; py -m unittest python.tests.test_sqlite_repositories.ProjectRepositoryScheduleTests`
  - passed with `64` tests.
