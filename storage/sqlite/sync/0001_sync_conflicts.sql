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

CREATE TABLE sync_conflicts (
    conflict_id TEXT PRIMARY KEY,
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
    kind TEXT NOT NULL CHECK (
        kind IN (
            'concurrent_update',
            'deleted_in_reference',
            'deleted_locally',
            'checksum_mismatch',
            'schema_mismatch'
        )
    ),
    local_snapshot TEXT NOT NULL,
    reference_snapshot TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('open', 'resolved', 'deferred')),
    resolution TEXT CHECK (
        resolution IN (
            'keep_local',
            'keep_reference',
            'manual_merge',
            'accept_deletion',
            'defer'
        )
    ),
    detected_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    CHECK (
        (status = 'open' AND resolution IS NULL)
        OR (status = 'deferred' AND resolution = 'defer')
        OR (
            status = 'resolved'
            AND resolution IN (
                'keep_local',
                'keep_reference',
                'manual_merge',
                'accept_deletion'
            )
        )
    )
);

CREATE TABLE sync_conflict_action_plans (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    conflict_id TEXT NOT NULL REFERENCES sync_conflicts(conflict_id),
    sequence INTEGER NOT NULL,
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
    kind TEXT NOT NULL CHECK (
        kind IN (
            'concurrent_update',
            'deleted_in_reference',
            'deleted_locally',
            'checksum_mismatch',
            'schema_mismatch'
        )
    ),
    resolution TEXT NOT NULL CHECK (
        resolution IN (
            'keep_local',
            'keep_reference',
            'manual_merge',
            'accept_deletion',
            'defer'
        )
    ),
    action TEXT NOT NULL CHECK (
        action IN (
            'push_local_snapshot',
            'pull_reference_snapshot',
            'manual_merge',
            'apply_deletion',
            'defer_for_review'
        )
    ),
    local_snapshot TEXT NOT NULL,
    reference_snapshot TEXT NOT NULL,
    requires_audit_event INTEGER NOT NULL DEFAULT 1 CHECK (
        requires_audit_event IN (0, 1)
    ),
    planned_by TEXT NOT NULL,
    planned_at TEXT NOT NULL,
    applied_at TEXT,
    audit_event_reference TEXT,
    UNIQUE(conflict_id, sequence)
);

CREATE TRIGGER sync_action_plan_matches_conflict
BEFORE INSERT ON sync_conflict_action_plans
FOR EACH ROW
WHEN NOT EXISTS (
    SELECT 1
    FROM sync_conflicts
    WHERE conflict_id = NEW.conflict_id
      AND domain = NEW.domain
      AND kind = NEW.kind
      AND local_snapshot = NEW.local_snapshot
      AND reference_snapshot = NEW.reference_snapshot
)
BEGIN
    SELECT RAISE(ABORT, 'sync action plan does not match conflict context');
END;

INSERT INTO repository_metadata(key, value, updated_at)
VALUES
    ('domain', 'sync', '1970-01-01T00:00:00Z'),
    ('sync_direction', 'local_only', '1970-01-01T00:00:00Z');

INSERT INTO schema_migrations(version, name, applied_at)
VALUES (1, 'sync_conflicts', '1970-01-01T00:00:00Z');
