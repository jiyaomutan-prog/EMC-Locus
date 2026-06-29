PRAGMA foreign_keys = ON;

CREATE TABLE sync_entity_snapshots (
    snapshot_id TEXT PRIMARY KEY,
    domain TEXT NOT NULL CHECK (
        domain IN (
            'metrology',
            'test_definitions',
            'instrument_drivers',
            'project_records',
            'measurement_data',
            'report_templates',
            'update_catalog'
        )
    ),
    entity_type TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    revision TEXT NOT NULL,
    snapshot_checksum TEXT NOT NULL,
    payload_json TEXT NOT NULL DEFAULT '{}',
    source_operation_id TEXT REFERENCES sync_operations(operation_id),
    captured_at TEXT NOT NULL,
    CHECK (
        length(snapshot_checksum) = 71
        AND substr(snapshot_checksum, 1, 7) = 'sha256:'
        AND substr(snapshot_checksum, 8) NOT GLOB '*[^0-9A-Fa-f]*'
    ),
    UNIQUE(domain, entity_type, entity_id, revision)
);

CREATE INDEX idx_sync_entity_snapshots_entity
ON sync_entity_snapshots(domain, entity_type, entity_id, captured_at);

CREATE TABLE sync_checkpoints (
    peer_id TEXT NOT NULL,
    domain TEXT NOT NULL CHECK (
        domain IN (
            'metrology',
            'test_definitions',
            'instrument_drivers',
            'project_records',
            'measurement_data',
            'report_templates',
            'update_catalog'
        )
    ),
    direction TEXT NOT NULL CHECK (
        direction IN ('push', 'pull', 'bidirectional')
    ),
    last_operation_id TEXT,
    last_snapshot_id TEXT,
    checkpoint_token TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    PRIMARY KEY(peer_id, domain, direction)
);

CREATE INDEX idx_sync_checkpoints_domain
ON sync_checkpoints(domain, direction, updated_at);

INSERT OR REPLACE INTO repository_metadata(key, value, updated_at)
VALUES
    ('entity_snapshot_schema', '2026-06-29-v1', '1970-01-01T00:00:00Z');

INSERT INTO schema_migrations(version, name, applied_at)
VALUES (3, 'entity_snapshots', '1970-01-01T00:00:00Z');
