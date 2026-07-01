# Test Template API

The 0.9.0 test-template API replaces the 0.8.x one-row template model with a
revisioned business aggregate owned by the Rust local agent.

The API manages reusable test definitions only. It does not instantiate a
campaign test, run instruments, acquire data, execute post-processing, or build
the future LAB CONSOLE editor.

## Vocabulary

- Template identity: stable `template_id`, title, category, creation metadata,
  and the pointer to the current approved revision.
- Template revision: immutable or draft content revision with deterministic
  `revision_number`, explicit `revision_id`, optional parent revision, status,
  canonical definition JSON, and SHA-256 checksum.
- Revision status: `draft`, `under_review`, `approved`, `suspended`,
  `superseded`, or `retired`. Only `draft` definitions are editable.
- Audit sequence: local append-only event order in `test_template_audit_events`.
  It is not a business revision number.
- Definition checksum: SHA-256 of the canonical, versioned definition JSON.

## Routes

```text
POST /api/v1/test-templates
GET  /api/v1/test-templates
GET  /api/v1/test-templates?category_code=emission_transient_time_domain
GET  /api/v1/test-templates/{template_id}

GET  /api/v1/test-templates/{template_id}/revisions
GET  /api/v1/test-templates/{template_id}/revisions/{revision_id}
PUT  /api/v1/test-templates/{template_id}/revisions/{revision_id}/definition

POST /api/v1/test-templates/{template_id}/revisions
POST /api/v1/test-templates/{template_id}/revisions/{revision_id}/transitions/submit-for-review
POST /api/v1/test-templates/{template_id}/revisions/{revision_id}/transitions/approve

GET  /api/v1/test-templates/{template_id}/audit-events
```

## Create A Template

Creation creates a template identity plus its first draft revision. The client
does not supply a revision number.

```json
{
  "template_id": "TT-INRUSH-001",
  "title": "Inrush current capture",
  "category_code": "emission_transient_time_domain",
  "definition": {
    "definition_schema_version": "emc-locus.test-template-definition.v1",
    "title": "Inrush current capture",
    "description": "Time-domain inrush capture for EMC investigations.",
    "measurement_axis": "time_series",
    "standard_references": ["IEC-61000-4-30"],
    "variables": [
      {
        "variable_id": "sample_rate_hz",
        "label": "Sample rate",
        "value_type": "number",
        "default_value": 100000.0,
        "constraints": {
          "required": true,
          "unit": "Hz",
          "minimum": 1000.0,
          "maximum": 1000000.0
        }
      }
    ],
    "lock_policy": [
      {
        "variable_id": "sample_rate_hz",
        "policy": "editable_until_campaign_freeze"
      }
    ],
    "instrumentation_chain": [
      {
        "slot_id": "daq",
        "label": "DAQ",
        "required_category": "daq_chassis",
        "required": true,
        "calibration_requirement": "if_used",
        "substitution_policy": "same_capability"
      }
    ],
    "entry_step_id": "capture",
    "sequence": [
      {
        "step_id": "capture",
        "order": 10,
        "kind": "acquire",
        "label": "Capture transient",
        "required_slots": ["daq"]
      }
    ],
    "limits": [
      {
        "limit_id": "peak_current",
        "kind": "scalar_threshold",
        "axis": "time_series",
        "unit": "A",
        "application_domain": "inrush",
        "source_reference": "method:TD-INRUSH:A",
        "threshold": 30.0,
        "variable_refs": ["sample_rate_hz"]
      }
    ],
    "post_processing": [
      {
        "operation_id": "peak",
        "order": 10,
        "operation_type": "peak",
        "inputs": ["raw.current"],
        "outputs": ["calculated.peak_current"],
        "parameters": {"absolute": true}
      }
    ],
    "method_parameters": {}
  },
  "actor": "method.author",
  "reason": "first controlled draft",
  "operation_id": "op-create-test-template"
}
```

The agent canonicalizes `definition`, validates the typed core invariants, and
stores the canonical JSON plus `definition_checksum`.

When `definition.method_code` and `definition.method_revision` are present, the
referenced method revision must already be approved.

## Replace A Draft Definition

Only draft revisions are editable. Replacement requires optimistic concurrency:

```json
{
  "expected_definition_checksum": "sha256:...",
  "definition": {},
  "actor": "method.author",
  "reason": "update draft before review",
  "operation_id": "op-replace-definition"
}
```

```text
PUT /api/v1/test-templates/TT-INRUSH-001/revisions/TT-INRUSH-001-rev-0001/definition
```

If the stored checksum differs from `expected_definition_checksum`, the agent
returns HTTP `409` with `test_template_definition_checksum_mismatch`.

If the revision is `under_review` or `approved`, the agent returns HTTP `409`
with `test_template_revision_immutable`.

## Transitions

Supported transitions in 0.9.0:

- `draft` -> `under_review`;
- `under_review` -> `approved`.

Submit:

```text
POST /api/v1/test-templates/TT-INRUSH-001/revisions/TT-INRUSH-001-rev-0001/transitions/submit-for-review
```

Approve:

```text
POST /api/v1/test-templates/TT-INRUSH-001/revisions/TT-INRUSH-001-rev-0001/transitions/approve
```

Both requests require:

```json
{
  "actor": "technical.reviewer",
  "reason": "technical review accepted",
  "operation_id": "op-approve-test-template"
}
```

The release does not implement authentication, RBAC, competence checks,
author/approver separation, or configurable second approval. Those belong to a
future people/roles/competence domain, so 0.9.0 records `actor`, `reason`, and
`operation_id` without imposing an arbitrary approval policy.

## Derive A New Revision

Approved revisions cannot be edited. To evolve a template, create a new draft
from an approved source:

```json
{
  "source_revision_id": "TT-INRUSH-001-rev-0001",
  "actor": "method.author",
  "reason": "prepare next method update",
  "operation_id": "op-create-template-rev2"
}
```

```text
POST /api/v1/test-templates/TT-INRUSH-001/revisions
```

The new revision receives the next deterministic revision number, a parent
revision reference, and a copied canonical definition. Historical revisions
remain readable.

## Response Shape

Write responses contain:

```json
{
  "operation": "test_template_created",
  "operation_id": "op-create-test-template",
  "replayed": false,
  "test_template": {
    "identity": {
      "template_id": "TT-INRUSH-001",
      "title": "Inrush current capture",
      "category_code": "emission_transient_time_domain",
      "current_approved_revision_id": null
    },
    "current_revision": {
      "revision_id": "TT-INRUSH-001-rev-0001",
      "revision_number": 1,
      "status": "draft",
      "definition_checksum": "sha256:..."
    }
  },
  "revision": {
    "revision_id": "TT-INRUSH-001-rev-0001",
    "status": "draft",
    "definition": {}
  }
}
```

## Idempotence

Every write route requires `operation_id`. Replaying the same canonical
operation returns `replayed=true`. Reusing an `operation_id` for a different
payload returns HTTP `409` with `operation_replay_mismatch`.

## Audit And Outbox

Each write appends `test_template_audit_events` with explicit:

- `template_id`;
- `revision_id`;
- `action`;
- `actor`;
- `reason`;
- old/new revision ids;
- old/new definition checksums;
- `operation_id`;
- `device_id`;
- `correlation_id`.

The sync outbox uses domain `test_definitions` and entity type
`test_template_revision`.

## Migration Policy

0.9.0 intentionally resets the 0.8.3/0.8.4 `test_templates` storage shape via
`storage/sqlite/test_definitions/0004_template_revision_aggregate.sql`. There
is no dual-read, dual-write, or legacy DTO in the runtime after migration.

## Simulated Execution Link

The simulated EMC execution route may still receive a stored `template_id` in
`test_method_reference`. When it matches a known template identity, the launch
requires `current_approved_revision_id` to be set. This is only a guardrail, not
an execution-package binding or a template instantiation engine.

## Limits

- No LAB CONSOLE template editor.
- No campaign test instantiation.
- No real acquisition, FFT, post-processing execution, or reporting.
- No authentication, RBAC, competence checks, second approval, or configurable
  approval policy.
- No file upload for template evidence.
