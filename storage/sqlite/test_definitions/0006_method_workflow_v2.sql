PRAGMA foreign_keys = ON;

CREATE TABLE method_hierarchy_nodes (
    node_id TEXT PRIMARY KEY,
    parent_node_id TEXT REFERENCES method_hierarchy_nodes(node_id),
    node_kind TEXT NOT NULL CHECK (node_kind IN (
        'domain', 'test_family', 'source_document', 'applicable_edition',
        'procedure', 'method_variant', 'parameter_profile', 'sub_range_profile'
    )),
    label TEXT NOT NULL,
    position INTEGER NOT NULL CHECK (position >= 0),
    archived INTEGER NOT NULL DEFAULT 0 CHECK (archived IN (0, 1)),
    revision INTEGER NOT NULL DEFAULT 1 CHECK (revision > 0),
    created_by TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    CHECK (trim(node_id) <> ''),
    CHECK (trim(label) <> ''),
    CHECK (parent_node_id IS NULL OR parent_node_id <> node_id)
);

CREATE INDEX method_hierarchy_parent_idx
ON method_hierarchy_nodes(parent_node_id, position, label);

CREATE TABLE method_workflow_identities (
    aggregate_kind TEXT NOT NULL CHECK (
        aggregate_kind IN ('measurement_system_template', 'regulation_profile')
    ),
    entity_id TEXT NOT NULL,
    label TEXT NOT NULL,
    classification TEXT NOT NULL,
    current_approved_revision_id TEXT,
    created_by TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    PRIMARY KEY (aggregate_kind, entity_id),
    CHECK (trim(entity_id) <> ''),
    CHECK (trim(label) <> '')
);

CREATE TABLE method_workflow_revisions (
    revision_id TEXT PRIMARY KEY,
    aggregate_kind TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    revision_number INTEGER NOT NULL CHECK (revision_number > 0),
    parent_revision_id TEXT REFERENCES method_workflow_revisions(revision_id),
    status TEXT NOT NULL CHECK (
        status IN ('draft', 'validated', 'approved', 'superseded', 'archived')
    ),
    definition_schema_version TEXT NOT NULL,
    definition_json TEXT NOT NULL,
    definition_checksum TEXT NOT NULL CHECK (
        length(definition_checksum) = 71
        AND substr(definition_checksum, 1, 7) = 'sha256:'
        AND definition_checksum = lower(definition_checksum)
        AND substr(definition_checksum, 8) NOT GLOB '*[^0-9a-f]*'
    ),
    created_by TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    validated_at TEXT,
    approved_at TEXT,
    UNIQUE (aggregate_kind, entity_id, revision_number),
    FOREIGN KEY (aggregate_kind, entity_id)
        REFERENCES method_workflow_identities(aggregate_kind, entity_id)
);

CREATE UNIQUE INDEX method_workflow_one_draft_idx
ON method_workflow_revisions(aggregate_kind, entity_id)
WHERE status = 'draft';

CREATE INDEX method_workflow_revision_history_idx
ON method_workflow_revisions(aggregate_kind, entity_id, revision_number DESC);

CREATE TABLE method_workflow_audit_events (
    audit_id INTEGER PRIMARY KEY AUTOINCREMENT,
    aggregate_kind TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    revision_id TEXT,
    action TEXT NOT NULL,
    actor TEXT NOT NULL,
    reason TEXT NOT NULL,
    old_definition_checksum TEXT,
    new_definition_checksum TEXT,
    operation_id TEXT NOT NULL UNIQUE,
    device_id TEXT NOT NULL,
    correlation_id TEXT NOT NULL,
    payload_json TEXT NOT NULL,
    payload_checksum TEXT NOT NULL,
    occurred_at TEXT NOT NULL
);

CREATE INDEX method_workflow_audit_entity_idx
ON method_workflow_audit_events(aggregate_kind, entity_id, audit_id);

CREATE TABLE method_workflow_operations (
    operation_id TEXT PRIMARY KEY,
    aggregate_kind TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    action TEXT NOT NULL,
    request_checksum TEXT NOT NULL,
    result_revision_id TEXT,
    result_definition_checksum TEXT,
    response_json TEXT NOT NULL,
    occurred_at TEXT NOT NULL
);

INSERT OR REPLACE INTO repository_metadata(key, value, updated_at)
VALUES
    ('test_method_definition_schema', 'emc-locus.test-method-definition.v2', '2026-08-05T00:00:00Z'),
    ('measurement_system_template_schema', 'emc-locus.measurement-system-template-definition.v1', '2026-08-05T00:00:00Z'),
    ('regulation_profile_schema', 'emc-locus.regulation-profile-definition.v1', '2026-08-05T00:00:00Z');

INSERT INTO schema_migrations(version, name, applied_at)
VALUES (6, 'method_workflow_v2', '2026-08-05T00:00:00Z');
