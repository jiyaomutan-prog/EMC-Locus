# Project API

The first local project API is served by `emc-locus-agent` on loopback only.

```text
cargo run -q -p emc-locus-agent -- serve --storage-root data\agent --migrations-root storage\sqlite --bind 127.0.0.1:8765
```

## Routes

```text
GET  /api/v1/health
POST /api/v1/storage/initialize
POST /api/v1/projects
GET  /api/v1/projects
GET  /api/v1/projects/{code}
GET  /api/v1/projects/{code}/contract-review
POST /api/v1/projects/{code}/contract-review/items/{item}/complete
POST /api/v1/projects/{code}/transitions/to-test-planning
GET  /api/v1/projects/{code}/audit-events
GET  /api/v1/sync/outbox
```

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

- a project or contract-review/stage update in `projects.sqlite`;
- a project audit event in `project_audit_events`;
- a pending `sync_operations` row in `sync.sqlite`.

This API is the local boundary for the project vertical slice. Metrology,
planning, documents, instrument control, and acquisition remain legacy or
separate work until they are explicitly migrated behind the agent.
