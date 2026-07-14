PRAGMA foreign_keys = ON;

CREATE TABLE schema_migrations (
    version INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    applied_at TEXT NOT NULL
);

CREATE TABLE repository_metadata (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE station_setup_identities (
    setup_id TEXT PRIMARY KEY,
    label TEXT NOT NULL,
    current_ready_revision_id TEXT REFERENCES station_setup_revisions(revision_id)
        DEFERRABLE INITIALLY DEFERRED,
    created_by TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE station_setup_revisions (
    revision_id TEXT PRIMARY KEY,
    setup_id TEXT NOT NULL REFERENCES station_setup_identities(setup_id),
    revision_number INTEGER NOT NULL CHECK (revision_number > 0),
    parent_revision_id TEXT REFERENCES station_setup_revisions(revision_id),
    status TEXT NOT NULL CHECK (status IN ('draft', 'ready', 'superseded')),
    definition_schema_version TEXT NOT NULL,
    definition_json TEXT NOT NULL,
    definition_checksum TEXT NOT NULL CHECK (
        length(definition_checksum) = 71
        AND substr(definition_checksum, 1, 7) = 'sha256:'
        AND definition_checksum = lower(definition_checksum)
        AND substr(definition_checksum, 8) NOT GLOB '*[^0-9a-f]*'
    ),
    readiness_json TEXT NOT NULL,
    created_by TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    ready_at TEXT,
    UNIQUE(setup_id, revision_number),
    CHECK (
        (status = 'draft' AND ready_at IS NULL)
        OR (status IN ('ready', 'superseded') AND ready_at IS NOT NULL)
    )
);

CREATE UNIQUE INDEX station_setup_one_draft_idx
ON station_setup_revisions(setup_id)
WHERE status = 'draft';

CREATE INDEX station_setup_revisions_history_idx
ON station_setup_revisions(setup_id, revision_number DESC);

CREATE TABLE station_setup_audit_events (
    audit_id INTEGER PRIMARY KEY AUTOINCREMENT,
    setup_id TEXT NOT NULL REFERENCES station_setup_identities(setup_id),
    revision_id TEXT REFERENCES station_setup_revisions(revision_id),
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
    payload_json TEXT NOT NULL DEFAULT '{}',
    payload_checksum TEXT NOT NULL CHECK (
        length(payload_checksum) = 71
        AND substr(payload_checksum, 1, 7) = 'sha256:'
        AND payload_checksum = lower(payload_checksum)
        AND substr(payload_checksum, 8) NOT GLOB '*[^0-9a-f]*'
    ),
    occurred_at TEXT NOT NULL
);

CREATE INDEX station_setup_audit_entity_idx
ON station_setup_audit_events(setup_id, audit_id);

CREATE TABLE station_setup_operations (
    operation_id TEXT PRIMARY KEY,
    setup_id TEXT NOT NULL,
    action TEXT NOT NULL,
    actor TEXT NOT NULL,
    device_id TEXT NOT NULL,
    correlation_id TEXT NOT NULL,
    payload_checksum TEXT NOT NULL CHECK (
        length(payload_checksum) = 71
        AND substr(payload_checksum, 1, 7) = 'sha256:'
        AND payload_checksum = lower(payload_checksum)
        AND substr(payload_checksum, 8) NOT GLOB '*[^0-9a-f]*'
    ),
    result_revision_id TEXT NOT NULL,
    result_definition_checksum TEXT NOT NULL,
    occurred_at TEXT NOT NULL
);

INSERT INTO repository_metadata(key, value, updated_at)
VALUES
    ('domain', 'station_configurations', '2026-07-14T00:00:00Z'),
    ('sync_direction', 'bidirectional', '2026-07-14T00:00:00Z'),
    ('station_setup_contract', 'revisioned-physical-measurement-setup-v1', '2026-07-14T00:00:00Z');

INSERT INTO schema_migrations(version, name, applied_at)
VALUES (1, 'station_measurement_setups', '2026-07-14T00:00:00Z');
