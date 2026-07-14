# Storage Schema Draft

This draft targets SQLite for an early single-workstation implementation. The
first versioned migrations now live under `storage/sqlite/` and are split by
domain repository.

## Principles

- Raw data is immutable after acquisition.
- Controlled metadata changes create audit events.
- Report output is linked back to datasets and review decisions.
- Instruments and calibration records are versioned enough to reconstruct the
  measurement context of a campaign.
- Application code should enforce domain invariants before writing rows.

## Tables

The original sketch below remains as a readable overview. The executable
migrations separate these tables into metrology, projects, test definitions,
equipment, measurement data, update catalog, and synchronization coordination
domains.

### equipment_model_identities

```sql
CREATE TABLE equipment_model_identities (
    equipment_model_id TEXT PRIMARY KEY,
    manufacturer TEXT NOT NULL,
    model_name TEXT NOT NULL,
    variant TEXT,
    equipment_class TEXT NOT NULL,
    category_code TEXT NOT NULL,
    current_approved_revision_id TEXT,
    created_by TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
```

### equipment_model_revisions

```sql
CREATE TABLE equipment_model_revisions (
    revision_id TEXT PRIMARY KEY,
    equipment_model_id TEXT NOT NULL,
    revision_number INTEGER NOT NULL,
    parent_revision_id TEXT,
    status TEXT NOT NULL,
    definition_schema_version TEXT NOT NULL,
    definition_json TEXT NOT NULL,
    definition_checksum TEXT NOT NULL,
    created_by TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    submitted_at TEXT,
    approved_at TEXT
);
```

Driver profile identities and revisions follow the same split, with driver
revisions referencing the supported approved equipment model revision and
checksum. Equipment audit events reference aggregate kind, entity id, revision
id, action, actor, reason, old/new revision, old/new checksum, operation id,
device id and correlation id.

### equipment physical classification registries

Release `0.12.0` adds canonical registry tables in `equipment.sqlite`:

- `equipment_functional_role_registry`;
- `equipment_signal_domain_registry`;
- `equipment_port_directionality_registry`;
- `equipment_flow_role_registry`;
- `equipment_technology_tag_registry`;
- `equipment_classification_presets`;
- `equipment_classification_preset_ports`.

These tables are the backend source of truth for model classification authoring
and LAB CONSOLE preset creation. Presets include default class, role, domains,
technology tags, and explicit port topology. They are not driver profiles and
do not execute hardware operations.

### equipment_model_classification_summaries

```sql
CREATE TABLE equipment_model_classification_summaries (
    equipment_model_id TEXT PRIMARY KEY,
    revision_id TEXT NOT NULL,
    revision_number INTEGER NOT NULL,
    status TEXT NOT NULL,
    manufacturer TEXT NOT NULL,
    equipment_class TEXT NOT NULL,
    category_code TEXT NOT NULL,
    functional_role TEXT NOT NULL,
    definition_checksum TEXT NOT NULL,
    signal_domains_json TEXT NOT NULL,
    technology_tags_json TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
```

The agent replaces this summary transactionally whenever a model definition is
created, edited, derived, submitted, or approved. Two normalized companion
tables, `equipment_model_signal_domain_summaries` and
`equipment_model_technology_tag_summaries`, support indexed role/domain/tag
catalog filters without parsing `definition_json`.

Release `0.13.1` extends this summary with `root_category_id` and `is_demo`
so LAB CONSOLE can browse by business taxonomy and hide demonstration records
without parsing revision JSON.

### equipment taxonomy and field templates

Migration `storage/sqlite/equipment/0004_equipment_taxonomy_field_templates.sql`
adds the equipment repository administration schema:

- `equipment_categories` stores root categories and subcategories with
  parent/root references, label, sort order, active flag, and system-defined
  flag.
- `equipment_field_definitions` stores the field dictionary for equipment
  forms, including data type, scope, defaults, choices, units, uniqueness and
  activation metadata.
- `equipment_category_field_rules` stores visibility, required state, display
  group/order, default values and help overrides for a category.
- `equipment_model_field_values` stores searchable revision-aware field values
  for a model revision.
- `equipment_model_template_snapshots` stores the effective entry-template
  snapshot captured with a model revision.

Fresh initialization seeds only structural defaults: seven root categories,
basic subcategories, a minimal field dictionary, and default field rules. No
demo models, sensors, drivers, or recipes are inserted by this migration.
Demo records are created only by explicit seed commands and must be marked as
demo data.

### measurement engineering definitions

Release `0.13.0` keeps reusable measurement-chain engineering definitions in
`equipment.sqlite`, because they describe what a class of sensor, correction
artifact, DAQ channel, or logical channel recipe is. They are not physical
serial-numbered assets and they are not runtime acquisitions.

The migration `storage/sqlite/equipment/0003_sensors_scaling_curves.sql`
adds the following revisioned aggregate tables:

- `sensor_definition_identities` and `sensor_definition_revisions`;
- `scaling_profile_identities` and `scaling_profile_revisions`;
- `engineering_curve_identities` and `engineering_curve_revisions`;
- `daq_channel_profile_identities` and `daq_channel_profile_revisions`;
- `acquisition_channel_recipe_identities` and
  `acquisition_channel_recipe_revisions`;
- `measurement_engineering_audit_events`.

Each identity table stores the stable entity id, label, summary kind, current
approved revision pointer, creator, and timestamps. Each revision table stores
the deterministic revision number, optional parent revision, lifecycle status,
canonical typed definition JSON, SHA-256 definition checksum, creator,
timestamps, and submit/approval evidence. The lifecycle supports `draft`,
`under_review`, `approved`, `superseded`, `suspended`, and `retired`.

The audit table records the aggregate kind, entity id, revision id, action,
actor, reason, old/new revision references, old/new definition checksums,
operation id, device id, correlation id, payload JSON, payload checksum, and
event time. This audit sequence is local event order only; the business content
history is carried by revision ids, revision numbers, parent links, statuses,
and definition checksums.

Engineering curves store 1D and future multi-axis correction artifacts as typed
canonical JSON. The current runtime supports deterministic 1D evaluation for
frequency-dependent artifacts; the table does not imply a real DAQ runtime,
station connection binding, or calibration-certificate file store.

Release `0.15.0` adds no table and no dual storage path. Backward-compatible
fields are carried in canonical revision JSON: sample conversions declare the
time-domain representation and optional input limits; frequency responses
declare amplitude/phase components and explicit operations; equipment-model
revisions declare optional signal paths. Each path references a controlled
conversion or response by identity, revision, and checksum. The agent resolves
those references in the same `equipment.sqlite` transaction boundary before a
model write or lifecycle transition is accepted.

### projects

```sql
CREATE TABLE projects (
    code TEXT PRIMARY KEY,
    customer_name TEXT NOT NULL,
    stage TEXT NOT NULL,
    created_at TEXT NOT NULL,
    archived_at TEXT
);
```

### project_audit_events

```sql
CREATE TABLE project_audit_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    project_code TEXT NOT NULL REFERENCES projects(code),
    sequence INTEGER NOT NULL,
    actor TEXT NOT NULL,
    action TEXT NOT NULL,
    reason TEXT,
    payload_json TEXT NOT NULL DEFAULT '{}',
    occurred_at TEXT NOT NULL,
    UNIQUE(project_code, sequence)
);
```

### contract_review_items

```sql
CREATE TABLE contract_review_items (
    project_code TEXT NOT NULL REFERENCES projects(code),
    item TEXT NOT NULL,
    completed INTEGER NOT NULL DEFAULT 0,
    completed_by TEXT,
    completed_at TEXT,
    comment TEXT,
    PRIMARY KEY (project_code, item)
);
```

### instruments

```sql
CREATE TABLE instruments (
    asset_id TEXT PRIMARY KEY,
    manufacturer TEXT NOT NULL,
    model TEXT NOT NULL,
    serial_number TEXT NOT NULL,
    family TEXT NOT NULL,
    status TEXT NOT NULL,
    calibration_requirement TEXT NOT NULL,
    capabilities_json TEXT NOT NULL DEFAULT '[]',
    UNIQUE(manufacturer, model, serial_number)
);
```

### calibration_records

```sql
CREATE TABLE calibration_records (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    asset_id TEXT NOT NULL REFERENCES instruments(asset_id),
    certificate_reference TEXT NOT NULL,
    calibrated_at TEXT NOT NULL,
    due_at TEXT NOT NULL,
    provider TEXT NOT NULL,
    status_at_import TEXT NOT NULL,
    uncertainty_json TEXT NOT NULL DEFAULT '{}',
    file_reference TEXT,
    checksum TEXT,
    UNIQUE(asset_id, certificate_reference)
);
```

### campaigns

```sql
CREATE TABLE campaigns (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    project_code TEXT NOT NULL REFERENCES projects(code),
    name TEXT NOT NULL,
    standard_reference TEXT NOT NULL,
    equipment_under_test TEXT NOT NULL,
    planned_at TEXT,
    started_at TEXT,
    completed_at TEXT
);
```

### measurement_runs

```sql
CREATE TABLE measurement_runs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    campaign_id INTEGER NOT NULL REFERENCES campaigns(id),
    operator TEXT NOT NULL,
    method_reference TEXT NOT NULL,
    software_version TEXT NOT NULL,
    environment_json TEXT NOT NULL DEFAULT '{}',
    started_at TEXT NOT NULL,
    completed_at TEXT
);
```

### asset_characterization_events

Migration `storage/sqlite/metrology/0009_asset_characterizations.sql` adds an
append-only result for a correction measured on one physical asset:

```sql
CREATE TABLE asset_characterization_events (
    characterization_id TEXT PRIMARY KEY,
    asset_id TEXT NOT NULL REFERENCES instruments(asset_id),
    characterization_kind TEXT NOT NULL,
    label TEXT NOT NULL,
    performed_on TEXT NOT NULL,
    valid_until TEXT NOT NULL,
    provider TEXT NOT NULL,
    method_reference TEXT NOT NULL,
    decision TEXT NOT NULL,
    definition_schema_version TEXT NOT NULL,
    definition_json TEXT NOT NULL,
    definition_checksum TEXT NOT NULL,
    certificate_reference TEXT,
    document_manifest_json TEXT,
    comment TEXT NOT NULL DEFAULT '',
    recorded_at TEXT NOT NULL,
    recorded_by TEXT NOT NULL,
    revision TEXT NOT NULL
);
```

`definition_json` is canonical typed JSON for either a time-sample conversion
or a frequency response. `definition_checksum` is a canonical prefixed SHA-256
digest. Corrections shared by all assets of a model remain revisioned in
`equipment.sqlite`; measured values for one serial number live only in this
metrology event table. The file manifest is metadata only. Uploaded bytes are
content-addressed below `objects/metrology/` rather than stored in SQLite.

Characterization insertion and its `metrology_audit_events` plus sync outbox
evidence are one transaction. The table is immutable in 0.16.0: a later
measurement creates another event instead of replacing the previous result.

### standards

```sql
CREATE TABLE standards (
    code TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    edition TEXT NOT NULL,
    issuer TEXT NOT NULL,
    status TEXT NOT NULL
);
```

### test_methods

```sql
CREATE TABLE test_methods (
    code TEXT PRIMARY KEY,
    standard_code TEXT REFERENCES standards(code),
    name TEXT NOT NULL,
    family TEXT NOT NULL,
    measurement_axis TEXT NOT NULL,
    controlled INTEGER NOT NULL DEFAULT 1
);
```

### test_method_revisions

```sql
CREATE TABLE test_method_revisions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    method_code TEXT NOT NULL REFERENCES test_methods(code),
    revision TEXT NOT NULL,
    status TEXT NOT NULL,
    parameters_json TEXT NOT NULL DEFAULT '{}',
    acceptance_criteria_json TEXT NOT NULL DEFAULT '{}',
    processing_graph_json TEXT NOT NULL DEFAULT '{}',
    approved_by TEXT,
    approved_at TEXT,
    checksum TEXT,
    UNIQUE(method_code, revision)
);
```

### test_steps

```sql
CREATE TABLE test_steps (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    method_revision_id INTEGER NOT NULL REFERENCES test_method_revisions(id),
    sequence INTEGER NOT NULL,
    name TEXT NOT NULL,
    instruction TEXT NOT NULL,
    expected_evidence TEXT NOT NULL,
    UNIQUE(method_revision_id, sequence)
);
```

### test_template_identities

```sql
CREATE TABLE test_template_identities (
    template_id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    category_code TEXT NOT NULL REFERENCES test_categories(code),
    current_approved_revision_id TEXT,
    created_by TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
```

### test_template_revisions

```sql
CREATE TABLE test_template_revisions (
    revision_id TEXT PRIMARY KEY,
    template_id TEXT NOT NULL REFERENCES test_template_identities(template_id),
    revision_number INTEGER NOT NULL,
    parent_revision_id TEXT REFERENCES test_template_revisions(revision_id),
    status TEXT NOT NULL,
    definition_schema_version TEXT NOT NULL,
    definition_json TEXT NOT NULL,
    definition_checksum TEXT NOT NULL,
    created_by TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    submitted_at TEXT,
    approved_at TEXT,
    UNIQUE(template_id, revision_number)
);

CREATE UNIQUE INDEX test_template_one_active_draft_idx
    ON test_template_revisions(template_id)
    WHERE status = 'draft';
```

`definition_json` is canonical JSON produced from the typed core definition.
`definition_checksum` is the SHA-256 content checksum of that canonical JSON.
The checksum is not an audit-event sequence and not a freely supplied client
revision string. The partial unique index enforces the 0.9.1 rule that a
template identity can have at most one active draft revision.

### test_template_audit_events

```sql
CREATE TABLE test_template_audit_events (
    audit_id INTEGER PRIMARY KEY AUTOINCREMENT,
    template_id TEXT NOT NULL REFERENCES test_template_identities(template_id),
    revision_id TEXT REFERENCES test_template_revisions(revision_id),
    action TEXT NOT NULL,
    actor TEXT NOT NULL,
    reason TEXT NOT NULL,
    old_revision_id TEXT,
    new_revision_id TEXT,
    old_definition_checksum TEXT,
    new_definition_checksum TEXT,
    operation_id TEXT NOT NULL UNIQUE,
    device_id TEXT NOT NULL,
    correlation_id TEXT NOT NULL,
    payload_json TEXT NOT NULL,
    payload_checksum TEXT NOT NULL,
    occurred_at TEXT NOT NULL
);
```

The audit id gives local event order only. Template content history is carried
by `test_template_revisions.revision_number`, `parent_revision_id`, and
definition checksums.

### measurement_run_instruments

```sql
CREATE TABLE measurement_run_instruments (
    measurement_run_id INTEGER NOT NULL REFERENCES measurement_runs(id),
    asset_id TEXT NOT NULL REFERENCES instruments(asset_id),
    calibration_record_id INTEGER NOT NULL REFERENCES calibration_records(id),
    role TEXT NOT NULL,
    PRIMARY KEY (measurement_run_id, asset_id, role)
);
```

### datasets

```sql
CREATE TABLE datasets (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    measurement_run_id INTEGER NOT NULL REFERENCES measurement_runs(id),
    kind TEXT NOT NULL,
    file_reference TEXT NOT NULL,
    checksum TEXT NOT NULL,
    acquired_at TEXT NOT NULL,
    immutable INTEGER NOT NULL DEFAULT 1,
    retention_status TEXT NOT NULL DEFAULT 'retained'
);
```

### dataset_retention_events

```sql
CREATE TABLE dataset_retention_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    dataset_id INTEGER NOT NULL REFERENCES datasets(id),
    previous_status TEXT NOT NULL,
    new_status TEXT NOT NULL,
    actor TEXT NOT NULL,
    reason TEXT NOT NULL,
    event_at TEXT NOT NULL,
    audit_event_reference TEXT
);
```

### processing_graph_instances

```sql
CREATE TABLE processing_graph_instances (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    source_dataset_id INTEGER NOT NULL REFERENCES datasets(id),
    graph_reference TEXT NOT NULL,
    graph_revision TEXT NOT NULL,
    operations_json TEXT NOT NULL,
    created_by TEXT NOT NULL,
    created_at TEXT NOT NULL,
    software_version TEXT NOT NULL,
    source_dataset_checksum TEXT,
    graph_checksum TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'active',
    UNIQUE(source_dataset_id, graph_reference, graph_revision)
);
```

### processing_graph_instance_artifacts

```sql
CREATE TABLE processing_graph_instance_artifacts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    processing_graph_instance_id INTEGER NOT NULL
        REFERENCES processing_graph_instances(id),
    output_signal_reference TEXT NOT NULL,
    artifact_kind TEXT NOT NULL,
    file_reference TEXT NOT NULL,
    checksum TEXT NOT NULL,
    created_at TEXT NOT NULL,
    raw_lineage_json TEXT NOT NULL DEFAULT '[]'
);
```

### processing_graph_executions

```sql
CREATE TABLE processing_graph_executions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    processing_graph_instance_id INTEGER NOT NULL
        REFERENCES processing_graph_instances(id),
    execution_reference TEXT NOT NULL,
    executed_by TEXT NOT NULL,
    executed_at TEXT NOT NULL,
    software_version TEXT NOT NULL,
    status TEXT NOT NULL,
    output_artifact_count INTEGER NOT NULL,
    notes TEXT,
    UNIQUE(processing_graph_instance_id, execution_reference)
);
```

Repository writes record graph artifacts and executions only for existing graph
instances. Artifact writes validate that output signal references use the
controlled signal-reference syntax and that raw-lineage evidence is a JSON array
of controlled signal references before persistence.
Processing graph writes reject `operations_json` values that are not valid JSON
objects or arrays before persistence.
Processing graph instance and execution writes reject blank software-version
evidence before persistence.
Every recorded execution must report an `output_artifact_count` that matches the
number of persisted `processing_graph_instance_artifacts` rows for the same
graph instance. A completed execution must have at least one output artifact.

### instrument_observations

```sql
CREATE TABLE instrument_observations (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    project_code TEXT NOT NULL,
    campaign_reference TEXT NOT NULL,
    measurement_run_reference TEXT NOT NULL,
    sequence INTEGER NOT NULL,
    instrument_code TEXT NOT NULL,
    transport TEXT NOT NULL,
    endpoint TEXT NOT NULL,
    command_message TEXT NOT NULL,
    response_message TEXT NOT NULL,
    success INTEGER NOT NULL,
    exchange_attempts INTEGER NOT NULL,
    observed_at TEXT NOT NULL,
    raw_payload_json TEXT NOT NULL DEFAULT '{}',
    observation_checksum TEXT,
    UNIQUE(measurement_run_reference, instrument_code, sequence)
);
```

### reports

```sql
CREATE TABLE reports (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    project_code TEXT NOT NULL REFERENCES projects(code),
    report_number TEXT NOT NULL,
    revision TEXT NOT NULL,
    status TEXT NOT NULL,
    reviewed_by TEXT,
    approved_by TEXT,
    issued_at TEXT,
    file_reference TEXT,
    checksum TEXT,
    UNIQUE(report_number, revision)
);
```

### sync_conflicts

```sql
CREATE TABLE sync_conflicts (
    conflict_id TEXT PRIMARY KEY,
    domain TEXT NOT NULL,
    kind TEXT NOT NULL,
    local_snapshot TEXT NOT NULL,
    reference_snapshot TEXT NOT NULL,
    status TEXT NOT NULL,
    resolution TEXT,
    detected_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
```

### sync_conflict_action_plans

```sql
CREATE TABLE sync_conflict_action_plans (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    conflict_id TEXT NOT NULL REFERENCES sync_conflicts(conflict_id),
    sequence INTEGER NOT NULL,
    resolution TEXT NOT NULL,
    action TEXT NOT NULL,
    requires_audit_event INTEGER NOT NULL DEFAULT 1,
    planned_by TEXT NOT NULL,
    planned_at TEXT NOT NULL,
    audit_event_reference TEXT,
    UNIQUE(conflict_id, sequence)
);
```

### update_install_validation_evidence

```sql
CREATE TABLE update_install_validation_evidence (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    package_name TEXT NOT NULL,
    package_version TEXT NOT NULL,
    component TEXT NOT NULL,
    installed_version TEXT NOT NULL,
    source TEXT NOT NULL,
    validation_status TEXT NOT NULL,
    signature_required INTEGER NOT NULL,
    signature_present INTEGER NOT NULL,
    compatibility_minimum_version TEXT NOT NULL,
    compatibility_maximum_version TEXT,
    package_offline_install_allowed INTEGER NOT NULL,
    policy_offline_install_allowed INTEGER NOT NULL,
    measurement_active INTEGER NOT NULL,
    apply_during_measurement_allowed INTEGER NOT NULL,
    reason TEXT,
    validated_by TEXT NOT NULL,
    validated_at TEXT NOT NULL
);
```

## Next Schema Questions

- Should timestamps be local laboratory time plus UTC offset, or UTC only?
- Which file storage convention should be used for raw data and reports?
- Should audit events use JSON payloads first, or strongly typed event tables?
- Which user and authorization model should be introduced before technical
  review and report approval?
