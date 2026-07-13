# 2026-07-13 - Agent Expected Checksum Validation Session

## Objective

Repair an API checksum validation inconsistency before starting new `0.14.0`
physical-asset scope.

## Work Completed

- Found that agent draft-replacement inputs could accept uppercase
  hexadecimal payloads in `expected_definition_checksum`, while stored
  definition checksums and domain documentation use canonical lowercase
  SHA-256 evidence.
- Tightened measurement-engineering, test-template, equipment-model, and
  driver-profile draft replacement validation to require
  `sha256:<64 lowercase hex characters>`.
- Added local API regression coverage proving uppercase expected checksums are
  rejected with validation errors before compare-and-swap writes.
- Updated `CHANGELOG.md`, `docs/equipment-api.md`,
  `docs/domain/template-and-execution-definition.md`, and
  `docs/local-agent.md` to document the canonical expected-checksum contract.

## Validation Notes

- `cargo fmt`: passed.
- `cargo test -p emc-locus-agent local_api_runs_measurement_engineering_workflow`: passed.
- `cargo test -p emc-locus-agent local_api_guards_equipment_validation_cas_and_immutability`: passed.
- `cargo test -p emc-locus-agent local_api_creates_test_template_with_audit_and_outbox`: passed.
- `cargo test -p emc-locus-agent local_api_runs_equipment_model_driver_and_simulation_workflow`: passed.
- `py -m compileall python/emc_locus`: passed.
- `cargo test`: passed (`48` agent tests and `201` core tests).
- `cargo fmt --check`: passed.
- `git diff --check`: passed, with expected Windows line-ending warnings only.

## Limits

This session only fixes agent-side expected-checksum validation for draft
replacement requests. It does not change SQLite schema, bump `VERSION`, create
a release tag, implement physical assets, station wiring, acquisition, FFT,
reports, RBAC, or central sync.
