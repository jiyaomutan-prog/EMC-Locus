PRAGMA foreign_keys = ON;

ALTER TABLE datasets
ADD COLUMN retention_status TEXT NOT NULL DEFAULT 'retained'
CHECK (
    retention_status IN (
        'retained',
        'deletion_requested',
        'deletion_approved',
        'deletion_rejected',
        'deleted'
    )
);

CREATE TABLE dataset_retention_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    dataset_id INTEGER NOT NULL REFERENCES datasets(id),
    previous_status TEXT NOT NULL CHECK (
        previous_status IN (
            'retained',
            'deletion_requested',
            'deletion_approved',
            'deletion_rejected',
            'deleted'
        )
    ),
    new_status TEXT NOT NULL CHECK (
        new_status IN (
            'retained',
            'deletion_requested',
            'deletion_approved',
            'deletion_rejected',
            'deleted'
        )
    ),
    actor TEXT NOT NULL,
    reason TEXT NOT NULL,
    event_at TEXT NOT NULL,
    audit_event_reference TEXT
);

CREATE INDEX idx_dataset_retention_events_dataset
ON dataset_retention_events(dataset_id, id);

INSERT INTO schema_migrations(version, name, applied_at)
VALUES (2, 'retention_evidence', '1970-01-01T00:00:00Z');
