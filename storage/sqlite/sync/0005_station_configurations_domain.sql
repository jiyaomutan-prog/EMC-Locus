PRAGMA foreign_keys = OFF;

CREATE TABLE sync_operations_v5 (
    operation_id TEXT PRIMARY KEY,
    domain TEXT NOT NULL CHECK (
        domain IN (
            'metrology', 'equipment', 'test_definitions', 'instrument_drivers',
            'project_records', 'station_configurations', 'measurement_data',
            'report_templates', 'update_catalog'
        )
    ),
    entity_type TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    operation_kind TEXT NOT NULL,
    base_revision TEXT NOT NULL,
    resulting_revision TEXT NOT NULL,
    actor_id TEXT NOT NULL,
    device_id TEXT NOT NULL,
    correlation_id TEXT NOT NULL,
    payload_json TEXT NOT NULL DEFAULT '{}',
    payload_checksum TEXT NOT NULL,
    status TEXT NOT NULL CHECK (
        status IN ('pending', 'applied', 'superseded', 'failed')
    ),
    occurred_at TEXT NOT NULL,
    recorded_at TEXT NOT NULL,
    applied_at TEXT,
    error_message TEXT,
    CHECK (base_revision <> resulting_revision),
    CHECK (
        length(payload_checksum) = 71
        AND substr(payload_checksum, 1, 7) = 'sha256:'
        AND substr(payload_checksum, 8) NOT GLOB '*[^0-9A-Fa-f]*'
    ),
    CHECK (
        (status IN ('pending', 'superseded') AND applied_at IS NULL)
        OR (status IN ('applied', 'failed') AND applied_at IS NOT NULL)
    )
);

INSERT INTO sync_operations_v5 SELECT * FROM sync_operations;
DROP TABLE sync_operations;
ALTER TABLE sync_operations_v5 RENAME TO sync_operations;

CREATE INDEX idx_sync_operations_entity
ON sync_operations(domain, entity_type, entity_id, recorded_at);

CREATE INDEX idx_sync_operations_status
ON sync_operations(status, recorded_at);

CREATE TABLE sync_entity_snapshots_v5 (
    snapshot_id TEXT PRIMARY KEY,
    domain TEXT NOT NULL CHECK (
        domain IN (
            'metrology', 'equipment', 'test_definitions', 'instrument_drivers',
            'project_records', 'station_configurations', 'measurement_data',
            'report_templates', 'update_catalog'
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

INSERT INTO sync_entity_snapshots_v5 SELECT * FROM sync_entity_snapshots;
DROP TABLE sync_entity_snapshots;
ALTER TABLE sync_entity_snapshots_v5 RENAME TO sync_entity_snapshots;

CREATE INDEX idx_sync_entity_snapshots_entity
ON sync_entity_snapshots(domain, entity_type, entity_id, captured_at);

CREATE TABLE sync_checkpoints_v5 (
    peer_id TEXT NOT NULL,
    domain TEXT NOT NULL CHECK (
        domain IN (
            'metrology', 'equipment', 'test_definitions', 'instrument_drivers',
            'project_records', 'station_configurations', 'measurement_data',
            'report_templates', 'update_catalog'
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

INSERT INTO sync_checkpoints_v5 SELECT * FROM sync_checkpoints;
DROP TABLE sync_checkpoints;
ALTER TABLE sync_checkpoints_v5 RENAME TO sync_checkpoints;

CREATE INDEX idx_sync_checkpoints_domain
ON sync_checkpoints(domain, direction, updated_at);

INSERT OR REPLACE INTO repository_metadata(key, value, updated_at)
VALUES
    ('station_configurations_domain_supported', 'true', '2026-07-14T00:00:00Z');

INSERT INTO schema_migrations(version, name, applied_at)
VALUES (5, 'station_configurations_domain', '2026-07-14T00:00:00Z');

PRAGMA foreign_keys = ON;
