PRAGMA foreign_keys = ON;

CREATE TABLE metrology_audit_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    entity_type TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    sequence INTEGER NOT NULL,
    actor TEXT NOT NULL,
    action TEXT NOT NULL,
    reason TEXT NOT NULL,
    operation_id TEXT NOT NULL,
    correlation_id TEXT NOT NULL,
    device_id TEXT NOT NULL,
    base_revision TEXT NOT NULL,
    resulting_revision TEXT NOT NULL,
    payload_json TEXT NOT NULL DEFAULT '{}',
    payload_checksum TEXT NOT NULL,
    occurred_at TEXT NOT NULL,
    UNIQUE(entity_type, entity_id, sequence),
    UNIQUE(operation_id),
    CHECK (base_revision <> resulting_revision),
    CHECK (
        length(payload_checksum) = 71
        AND substr(payload_checksum, 1, 7) = 'sha256:'
        AND substr(payload_checksum, 8) NOT GLOB '*[^0-9A-Fa-f]*'
    )
);

CREATE INDEX metrology_audit_events_entity_idx
    ON metrology_audit_events(entity_type, entity_id, sequence);

INSERT OR REPLACE INTO repository_metadata(key, value, updated_at)
VALUES
    ('metrology_audit_schema', '2026-06-30-v1', '2026-06-30T00:00:00Z');

INSERT INTO schema_migrations(version, name, applied_at)
VALUES (6, 'metrology_audit_events', '2026-06-30T00:00:00Z');
