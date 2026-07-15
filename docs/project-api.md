# Project API

The first local project API is served by `emc-locus-agent` on loopback only.

```text
cargo run -q -p emc-locus-agent -- serve --storage-root data\agent --migrations-root storage\sqlite --bind 127.0.0.1:8765
```

## Routes

```text
GET  /api/v1/health
GET  /api/v1/storage/status
POST /api/v1/storage/initialize
POST /api/v1/projects
GET  /api/v1/projects
GET  /api/v1/projects/{code}
GET  /api/v1/projects/{code}/contract-review
POST /api/v1/projects/{code}/contract-review/items/{item}/complete
POST /api/v1/projects/{code}/transitions/to-test-planning
GET  /api/v1/projects/{code}/schedule-items
POST /api/v1/projects/{code}/schedule-items
POST /api/v1/projects/{code}/schedule-items/{item_code}/transitions/{action}
GET  /api/v1/projects/{code}/audit-events
GET  /api/v1/projects/{code}/test-executions
GET  /api/v1/documents
POST /api/v1/documents
GET  /api/v1/documents/{document_id}
GET  /api/v1/documents/{document_id}/audit-events
POST /api/v1/test-executions/simulated-emc
GET  /api/v1/test-executions/{attempt_id}
GET  /api/v1/sync/outbox
```

## Storage Status

`GET /api/v1/storage/status` returns the same project/sync schema report as the
agent CLI `storage status` command. Qt uses it to show whether the local agent
is connected, storage is missing, a migration is required, or integrity checks
are failing. Each domain also reports `journal_mode` and
`atomicity_compatible`; the project vertical slice rejects incompatible modes
such as `wal` because project and outbox writes span attached SQLite databases.

## Create Project

```json
{
  "code": "CEM-2026-001",
  "customer_name": "Example Customer",
  "execution_mode": "accredited",
  "actor": "quality.lead",
  "reason": "Contract accepted",
  "operation_id": "op-create-CEM-2026-001",
  "correlation_id": "corr-CEM-2026-001",
  "device_id": "station-a"
}
```

`stage` is optional and defaults to `contract_review` for the vertical slice.

## Complete Review Item

```json
{
  "actor": "quality.lead",
  "comment": "Customer request reviewed",
  "operation_id": "op-review-001"
}
```

The accredited checklist item slugs are:

```text
customer_request_defined
test_method_selected
laboratory_capability_confirmed
equipment_availability_checked
calibration_status_reviewed
impartiality_risks_reviewed
data_retention_agreed
report_requirements_agreed
deviations_recorded
```

## Transition To Planning

```json
{
  "actor": "quality.lead",
  "reason": "Contract review complete",
  "operation_id": "op-plan-CEM-2026-001"
}
```

If the contract review is incomplete, the API returns HTTP `409` with a
structured error:

```json
{
  "error": {
    "code": "contract_review_incomplete",
    "message": "Contract review is incomplete",
    "details": {
      "missing_items": [
        "customer_request_defined"
      ]
    }
  }
}
```

## Idempotence And Outbox

Write routes require `operation_id`. Replaying the same operation returns the
stored result without duplicating project, audit, or outbox rows.

Each successful write creates:

- a project, contract-review, stage, or schedule update in `projects.sqlite`;
- a project audit event in `project_audit_events`;
- a pending `sync_operations` row in `sync.sqlite`.

These writes require rollback-journal modes (`delete`, `truncate`, or
`persist`) on both SQLite files. `storage init` sets `journal_mode=DELETE`, and
project commands return a structured `storage_journal_mode_incompatible` error
if an operator or external tool switches either file to an incompatible mode.

This API is the local boundary for the project and first-slot planning vertical
slice. Instrument control and acquisition remain separate work until they are
explicitly migrated behind the agent. Attached document metadata is a shared
agent-owned registry; see `document-api.md`.

Qt/Python clients configured with `agent_url` use these routes for migrated
project and planning reads and writes. They must not open `projects.sqlite`
directly for project list/detail, contract-review status, service schedule,
project audit events, or sync outbox data.

## Plan A Test Slot

Planning is project-centred and becomes available only after the contract
review has moved the project to `test_planning`.

```json
{
  "item_code": "PLAN-CEM-2026-001-001",
  "title": "Conducted emission",
  "planned_start_at": "2026-07-15T09:00",
  "planned_end_at": "2026-07-15T12:00",
  "assigned_operator": "Alice Martin",
  "location": "EMC laboratory 1",
  "equipment_under_test": "Railway converter",
  "notes": "First agreed slot",
  "actor": "laboratory.manager",
  "reason": "Test slot agreed with the project team",
  "operation_id": "op-plan-CEM-2026-001-001"
}
```

The date-times are local canonical `YYYY-MM-DDTHH:MM` values. A slot must stay
inside one business day and start as `planned`. Active slots reserve both their
operator and location. Overlap errors return the conflicting slot and resource
as structured details; adjacent slots remain allowed.

Status actions are `confirm`, `start`, `complete`, and `cancel`. A transition
requires the current `expected_revision`; a stale revision returns HTTP `409`
without writing a partial audit or outbox operation.

## Simulated EMC Execution

Version `0.8.0` adds the first project-owned execution workflow. The operator
submits one launch attempt; the agent computes metrology readiness in the
context of that test, persists the attempt, and either refuses execution or
records a deterministic simulated result.

```json
{
  "attempt_id": "RUN-SIM-001",
  "project_code": "CEM-2026-001",
  "test_method_reference": "SIM-EMC-CONDUCTED",
  "execution_mode": "accredited",
  "required_asset_ids": ["SA-001"],
  "operator": "operator.one",
  "checked_on": "2026-07-01",
  "reason": "Operator launch",
  "operation_id": "op-run-sim-001"
}
```

If the preflight blocks, HTTP still returns `200` because the refused attempt is
valid persisted evidence:

```json
{
  "execution": {
    "attempt_id": "RUN-SIM-001",
    "status": "refused",
    "refusal": {
      "code": "equipment_readiness_blocked",
      "message": "Execution refused because required instrumentation is not ready",
      "causes": [
        {
          "asset_id": "SA-001",
          "dimension": "missing_evidence",
          "code": "calibration_missing",
          "message": "required calibration is not valid"
        }
      ]
    }
  }
}
```

If the preflight passes, the same route stores `status=completed` and a
`simulation_result` using the deterministic conducted-emission level-sweep
strategy. Both refused and completed attempts write:

- a row in `simulated_test_executions`;
- one or more rows in `simulated_test_execution_instruments`;
- a project audit event;
- a pending `sync_operations` row with entity type
  `simulated_test_execution`.
