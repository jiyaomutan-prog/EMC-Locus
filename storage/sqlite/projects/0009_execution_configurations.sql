PRAGMA foreign_keys = ON;

CREATE TABLE execution_configuration_identities (
    configuration_id TEXT PRIMARY KEY,
    project_code TEXT NOT NULL REFERENCES projects(code),
    schedule_item_code TEXT NOT NULL REFERENCES service_schedule_items(item_code),
    current_revision_id TEXT,
    created_by TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE(project_code, schedule_item_code)
);

CREATE TABLE execution_configuration_revisions (
    revision_id TEXT PRIMARY KEY,
    configuration_id TEXT NOT NULL REFERENCES execution_configuration_identities(configuration_id),
    revision_number INTEGER NOT NULL CHECK (revision_number > 0),
    parent_revision_id TEXT REFERENCES execution_configuration_revisions(revision_id),
    method_template_id TEXT NOT NULL,
    method_revision_id TEXT NOT NULL,
    method_definition_checksum TEXT NOT NULL,
    system_template_id TEXT NOT NULL,
    system_template_revision_id TEXT NOT NULL,
    system_template_definition_checksum TEXT NOT NULL,
    station_setup_id TEXT NOT NULL,
    station_setup_revision_id TEXT NOT NULL,
    station_setup_definition_checksum TEXT NOT NULL,
    planned_preparation_revision_id TEXT,
    definition_schema_version TEXT NOT NULL,
    definition_json TEXT NOT NULL,
    definition_checksum TEXT NOT NULL,
    readiness_json TEXT NOT NULL,
    operation_id TEXT NOT NULL UNIQUE,
    request_checksum TEXT NOT NULL,
    actor TEXT NOT NULL,
    reason TEXT NOT NULL,
    device_id TEXT NOT NULL,
    correlation_id TEXT NOT NULL,
    created_at TEXT NOT NULL,
    UNIQUE(configuration_id, revision_number),
    CHECK (definition_schema_version = 'emc-locus.execution-configuration.v1')
);

CREATE INDEX execution_configuration_history_idx
ON execution_configuration_revisions(configuration_id, revision_number DESC);

CREATE INDEX execution_configuration_schedule_idx
ON execution_configuration_identities(project_code, schedule_item_code);

INSERT OR REPLACE INTO repository_metadata(key, value, updated_at)
VALUES
    ('execution_configuration_schema', 'emc-locus.execution-configuration.v1', '2026-08-05T00:00:00Z'),
    ('execution_configuration_assignment_source', 'planned-test-preparation-v1', '2026-08-05T00:00:00Z');

INSERT INTO schema_migrations(version, name, applied_at)
VALUES (9, 'execution_configurations', '2026-08-05T00:00:00Z');
