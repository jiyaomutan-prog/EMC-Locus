PRAGMA foreign_keys = ON;

CREATE TABLE sync_operations (
    operation_id TEXT PRIMARY KEY,
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

CREATE INDEX idx_sync_operations_entity
ON sync_operations(domain, entity_type, entity_id, recorded_at);

CREATE INDEX idx_sync_operations_status
ON sync_operations(status, recorded_at);

INSERT OR REPLACE INTO repository_metadata(key, value, updated_at)
VALUES
    ('operation_journal_schema', '2026-06-29-v1', '1970-01-01T00:00:00Z');

INSERT INTO schema_migrations(version, name, applied_at)
VALUES (2, 'operation_journal', '1970-01-01T00:00:00Z');
