PRAGMA foreign_keys = ON;

CREATE TABLE planned_test_preparation_identities (
    project_code TEXT NOT NULL REFERENCES projects(code),
    schedule_item_code TEXT NOT NULL UNIQUE REFERENCES service_schedule_items(item_code),
    current_revision_id TEXT REFERENCES planned_test_preparation_revisions(revision_id)
        DEFERRABLE INITIALLY DEFERRED,
    created_by TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    PRIMARY KEY (project_code, schedule_item_code),
    UNIQUE (project_code, schedule_item_code)
);

CREATE TABLE planned_test_preparation_revisions (
    revision_id TEXT PRIMARY KEY,
    project_code TEXT NOT NULL,
    schedule_item_code TEXT NOT NULL,
    revision_number INTEGER NOT NULL CHECK (revision_number > 0),
    parent_revision_id TEXT REFERENCES planned_test_preparation_revisions(revision_id),
    schedule_revision INTEGER NOT NULL CHECK (schedule_revision > 0),
    method_template_id TEXT NOT NULL,
    method_revision_id TEXT NOT NULL,
    method_definition_checksum TEXT NOT NULL CHECK (
        length(method_definition_checksum) = 71
        AND substr(method_definition_checksum, 1, 7) = 'sha256:'
        AND method_definition_checksum = lower(method_definition_checksum)
        AND substr(method_definition_checksum, 8) NOT GLOB '*[^0-9a-f]*'
    ),
    station_setup_id TEXT NOT NULL,
    station_setup_revision_id TEXT NOT NULL,
    station_setup_definition_checksum TEXT NOT NULL CHECK (
        length(station_setup_definition_checksum) = 71
        AND substr(station_setup_definition_checksum, 1, 7) = 'sha256:'
        AND station_setup_definition_checksum = lower(station_setup_definition_checksum)
        AND substr(station_setup_definition_checksum, 8) NOT GLOB '*[^0-9a-f]*'
    ),
    verdict_state TEXT NOT NULL CHECK (verdict_state IN ('blocked', 'ready')),
    definition_schema_version TEXT NOT NULL,
    definition_json TEXT NOT NULL,
    definition_checksum TEXT NOT NULL CHECK (
        length(definition_checksum) = 71
        AND substr(definition_checksum, 1, 7) = 'sha256:'
        AND definition_checksum = lower(definition_checksum)
        AND substr(definition_checksum, 8) NOT GLOB '*[^0-9a-f]*'
    ),
    operation_id TEXT NOT NULL UNIQUE,
    request_checksum TEXT NOT NULL CHECK (
        length(request_checksum) = 71
        AND substr(request_checksum, 1, 7) = 'sha256:'
        AND request_checksum = lower(request_checksum)
        AND substr(request_checksum, 8) NOT GLOB '*[^0-9a-f]*'
    ),
    actor TEXT NOT NULL,
    reason TEXT NOT NULL,
    device_id TEXT NOT NULL,
    correlation_id TEXT NOT NULL,
    created_at TEXT NOT NULL,
    UNIQUE (project_code, schedule_item_code, revision_number),
    FOREIGN KEY (project_code, schedule_item_code)
        REFERENCES planned_test_preparation_identities(project_code, schedule_item_code)
        DEFERRABLE INITIALLY DEFERRED
);

CREATE INDEX planned_test_preparation_history_idx
ON planned_test_preparation_revisions(project_code, schedule_item_code, revision_number DESC);

CREATE INDEX planned_test_preparation_schedule_revision_idx
ON planned_test_preparation_revisions(schedule_item_code, schedule_revision);

CREATE INDEX planned_test_preparation_method_idx
ON planned_test_preparation_revisions(method_template_id, method_revision_id);

CREATE INDEX planned_test_preparation_station_idx
ON planned_test_preparation_revisions(station_setup_id, station_setup_revision_id);

INSERT OR REPLACE INTO repository_metadata(key, value, updated_at)
VALUES
    ('planned_test_preparation_schema', '2026-07-15-v1', '2026-07-15T00:00:00Z'),
    ('planned_test_preparation_writer', 'emc-locus-agent', '2026-07-15T00:00:00Z');

INSERT INTO schema_migrations(version, name, applied_at)
VALUES (7, 'planned_test_preparations', '2026-07-15T00:00:00Z');
