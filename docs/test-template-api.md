# Test Template API

The test-template API is the first agent-owned slice for executable test
definitions.

It stores controlled test templates in `test_definitions.sqlite` and emits
audit plus outbox evidence through `sync.sqlite`. It can create draft templates,
submit them for review, and approve reviewed templates. It does not yet
instantiate campaign test instances, execute tests, drive instruments, acquire
data, or run post-processing.

## Routes

```text
POST /api/v1/test-templates
GET  /api/v1/test-templates
GET  /api/v1/test-templates?category_code=emission_transient_time_domain&status=draft
GET  /api/v1/test-templates/{template_id}
GET  /api/v1/test-templates/{template_id}/audit-events
POST /api/v1/test-templates/{template_id}/transitions/submit-for-review
POST /api/v1/test-templates/{template_id}/transitions/approve
```

## Create Draft Test Template

```json
{
  "template_id": "TT-INRUSH-001",
  "template_revision": "A",
  "title": "Inrush current capture",
  "description": "Time-domain inrush capture template.",
  "category_code": "emission_transient_time_domain",
  "measurement_axis": "time_series",
  "variables": {
    "sample_rate_hz": {
      "type": "number",
      "unit": "Hz",
      "default": 100000
    }
  },
  "lock_policy": {
    "sample_rate_hz": "editable_until_campaign_freeze"
  },
  "instrumentation_chain": [
    {
      "slot": "current_probe",
      "required_category": "current_probe",
      "calibration": "required"
    },
    {
      "slot": "daq",
      "required_category": "daq_chassis",
      "sync": "single_clock"
    }
  ],
  "sequence": [
    {
      "step": "arm",
      "instruction": "Arm acquisition and wait for trigger"
    },
    {
      "step": "capture",
      "instruction": "Capture inrush transient"
    }
  ],
  "limits": [
    {
      "name": "peak_current",
      "expression": "max(abs(current))",
      "unit": "A"
    }
  ],
  "post_processing": [
    {
      "operation": "peak",
      "input": "raw.current",
      "output": "calculated.peak_current"
    }
  ],
  "actor": "method.author",
  "reason": "first controlled template draft",
  "operation_id": "op-create-test-template"
}
```

`template_revision` defaults to `A` when omitted. `status` defaults to `draft`;
new templates must start as `draft`.

The six structured definition fields are mandatory:

- `variables`, a JSON object;
- `lock_policy`, a JSON object;
- `instrumentation_chain`, a JSON array;
- `sequence`, a JSON array;
- `limits`, a JSON array;
- `post_processing`, a JSON array.

The API also accepts string fields ending with `_json` for callers that already
hold canonical JSON strings:

- `variables_json`;
- `lock_policy_json`;
- `instrumentation_chain_json`;
- `sequence_json`;
- `limits_json`;
- `post_processing_json`.

## Method Reference

`method_code` and `method_revision` are optional in this first slice, but they
must be provided together. When present, the referenced method revision must
already exist in `test_method_revisions` with status `approved`.

This lets a laboratory author early draft templates before the method lifecycle
is migrated to the agent, while still validating method links when they exist.

## Lifecycle Transitions

Submit a draft template for review:

```json
{
  "actor": "method.author",
  "reason": "definition ready for technical review",
  "operation_id": "op-submit-test-template"
}
```

```text
POST /api/v1/test-templates/TT-INRUSH-001/transitions/submit-for-review
```

Approve an under-review template:

```json
{
  "actor": "technical.reviewer",
  "reason": "technical review accepted",
  "operation_id": "op-approve-test-template"
}
```

```text
POST /api/v1/test-templates/TT-INRUSH-001/transitions/approve
```

The first supported lifecycle is intentionally small:

- `draft` -> `under_review`;
- `under_review` -> `approved`.

Direct approval from `draft`, re-approval of an already approved template, and
other unsupported moves return HTTP `409` with
`test_template_transition_not_allowed` and structured details containing the
current status, requested target, and allowed transitions.

Every transition requires `actor`, `reason`, and `operation_id`. Optional
`correlation_id` and `device_id` behave like other agent write routes.

## Response

Successful creation returns:

```json
{
  "operation": "test_template_created",
  "operation_id": "op-create-test-template",
  "replayed": false,
  "test_template": {
    "template_id": "TT-INRUSH-001",
    "template_revision": "A",
    "status": "draft",
    "variables": {},
    "instrumentation_chain": []
  }
}
```

The shown `test_template` is abbreviated here. The real response includes every
stored field and all structured definition JSON values.

## Idempotence

`operation_id` is required. Replaying the same canonical operation returns the
stored template with `replayed=true`. Reusing the same `operation_id` for a
different payload returns HTTP `409` with `operation_replay_mismatch`.

## Audit And Outbox

Each successful creation writes:

- one `test_templates` row;
- one `test_template_audit_events` row with action `test_template_created`;
- one pending `sync_operations` row with:
  - `domain = test_definitions`;
  - `entity_type = test_template`;
  - `operation_kind = test_template_created`.

Each successful lifecycle transition updates the existing `test_templates`
status, writes one `test_template_audit_events` row, and emits one pending
`sync_operations` row with one of these operation kinds:

- `test_template_submitted_for_review`;
- `test_template_approved`.

## Limits

- No configurable second-approval workflow.
- No suspension, retirement, or supersession route yet.
- No project/campaign instantiation.
- No method authoring route.
- No real acquisition or processing execution.
- No document attachment shortcut; use the attached-document registry for
  controlled files.
