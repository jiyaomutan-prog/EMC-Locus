# Test Method And Workflow API

Release 0.22.2 keeps the revisioned test-template identity API and adds method
v2, hierarchy, reusable topology, regulation and execution-plan routes. All
mutations require the repository operation context: `actor`, `reason`,
`operation_id`, `device_id` and `correlation_id`.

## Method Revisions

```text
POST /api/v1/test-templates
GET  /api/v1/test-templates
GET  /api/v1/test-templates/{template_id}
GET  /api/v1/test-templates/{template_id}/revisions
GET  /api/v1/test-templates/{template_id}/revisions/{revision_id}
PUT  /api/v1/test-templates/{template_id}/revisions/{revision_id}/definition
POST /api/v1/test-templates/{template_id}/revisions
POST /api/v1/test-templates/{template_id}/revisions/successor-0.22.2
POST /api/v1/test-templates/{template_id}/revisions/{revision_id}/transitions/submit-for-review
POST /api/v1/test-templates/{template_id}/revisions/{revision_id}/transitions/approve
GET  /api/v1/test-templates/{template_id}/audit-events
```

Draft replacement requires `expected_definition_checksum`. The successor route
is the only v1-to-v2 conversion path and returns a reviewable draft.

## Method Hierarchy

```text
GET  /api/v1/method-hierarchy
POST /api/v1/method-hierarchy
PUT  /api/v1/method-hierarchy/{node_id}
```

The update command uses the expected node revision and covers rename, move,
reorder, archive and restore. Stable node IDs do not depend on labels.

## Reusable Workflow Aggregates

For `{collection}` equal to `measurement-system-templates` or
`regulation-profiles`:

```text
GET  /api/v1/{collection}
POST /api/v1/{collection}
GET  /api/v1/{collection}/{entity_id}
GET  /api/v1/{collection}/{entity_id}/revisions/{revision_id}
POST /api/v1/{collection}/{entity_id}/revisions
PUT  /api/v1/{collection}/{entity_id}/revisions/{revision_id}/definition
POST /api/v1/{collection}/{entity_id}/revisions/{revision_id}/transitions/validate
POST /api/v1/{collection}/{entity_id}/revisions/{revision_id}/transitions/approve
GET  /api/v1/{collection}/{entity_id}/audit-events
```

Topology commands may include `method_definition`; topology commands with loop
mappings also include the referenced `regulation_profiles`. These are
validation context, not duplicated persistence.

## Preview And Dated Derivation

```text
POST /api/v1/sub-ranges/preview
POST /api/v1/execution-plans/preview
POST /api/v1/projects/{project_code}/schedule-items/{item_code}/execution-configuration
GET  /api/v1/execution-configurations/{configuration_id}
```

Sub-range preview has an explicit `maximum_points` and never expands an
unbounded browser array. Execution-plan preview is read-only. Dated derivation
requires an authoritative planned-preparation revision and exact immutable
method/system/station pins.

## Errors And Boundaries

Validation errors return stable codes, a human message and structured details.
Checksum or revision conflicts are explicit; failed mutations create no audit
or outbox records. The API defines and previews contracts only. No route in
this surface controls instruments, acquires data, executes FFT or applies
runtime corrections.
