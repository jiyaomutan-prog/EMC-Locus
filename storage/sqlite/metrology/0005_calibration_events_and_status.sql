PRAGMA foreign_keys = ON;

ALTER TABLE instruments
ADD COLUMN calibration_due_warning_days INTEGER NOT NULL DEFAULT 30 CHECK (
    calibration_due_warning_days > 0
);

CREATE TABLE calibration_events (
    event_id TEXT PRIMARY KEY,
    asset_id TEXT NOT NULL REFERENCES instruments(asset_id),
    certificate_reference TEXT NOT NULL,
    calibrated_at TEXT NOT NULL,
    due_at TEXT NOT NULL,
    provider TEXT NOT NULL,
    decision TEXT NOT NULL CHECK (
        decision IN ('conforming', 'nonconforming', 'indeterminate', 'not_assessed')
    ),
    as_found_status TEXT CHECK (
        as_found_status IS NULL OR
        as_found_status IN ('conforming', 'nonconforming', 'indeterminate', 'not_assessed')
    ),
    as_left_status TEXT CHECK (
        as_left_status IS NULL OR
        as_left_status IN ('conforming', 'nonconforming', 'indeterminate', 'not_assessed')
    ),
    adjustment_performed INTEGER NOT NULL DEFAULT 0 CHECK (
        adjustment_performed IN (0, 1)
    ),
    uncertainty_summary_json TEXT NOT NULL DEFAULT '{}',
    traceability_reference TEXT,
    comment TEXT NOT NULL DEFAULT '',
    document_manifest_json TEXT,
    recorded_at TEXT NOT NULL,
    recorded_by TEXT NOT NULL,
    revision TEXT NOT NULL,
    UNIQUE(asset_id, certificate_reference)
);

CREATE INDEX calibration_events_asset_due_idx
    ON calibration_events(asset_id, due_at, calibrated_at);

INSERT OR REPLACE INTO repository_metadata(key, value, updated_at)
VALUES
    ('calibration_status_policy', 'computed-from-events-v1', '2026-06-30T00:00:00Z'),
    ('default_calibration_due_warning_days', '30', '2026-06-30T00:00:00Z');

INSERT INTO schema_migrations(version, name, applied_at)
VALUES (5, 'calibration_events_and_status', '2026-06-30T00:00:00Z');
