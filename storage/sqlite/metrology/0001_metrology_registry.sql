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

CREATE TABLE instruments (
    asset_id TEXT PRIMARY KEY,
    family TEXT NOT NULL,
    manufacturer TEXT NOT NULL,
    model TEXT NOT NULL,
    serial_number TEXT NOT NULL,
    availability TEXT NOT NULL CHECK (
        availability IN ('available', 'reserved', 'out_of_service')
    ),
    calibration_requirement TEXT NOT NULL CHECK (
        calibration_requirement IN ('required', 'conditional', 'not_required')
    ),
    capabilities_json TEXT NOT NULL DEFAULT '[]',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE(manufacturer, model, serial_number)
);

CREATE TABLE calibration_records (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    asset_id TEXT NOT NULL REFERENCES instruments(asset_id),
    certificate_reference TEXT NOT NULL,
    calibrated_at TEXT NOT NULL,
    due_at TEXT NOT NULL,
    provider TEXT NOT NULL,
    status_at_import TEXT NOT NULL CHECK (
        status_at_import IN ('valid', 'due_soon', 'expired', 'missing', 'not_required')
    ),
    uncertainty_json TEXT NOT NULL DEFAULT '{}',
    file_reference TEXT,
    checksum TEXT,
    created_at TEXT NOT NULL,
    UNIQUE(asset_id, certificate_reference)
);

CREATE INDEX calibration_records_asset_due_idx
    ON calibration_records(asset_id, due_at);

INSERT INTO repository_metadata(key, value, updated_at)
VALUES
    ('domain', 'metrology', '1970-01-01T00:00:00Z'),
    ('sync_direction', 'pull_from_reference', '1970-01-01T00:00:00Z');

INSERT INTO schema_migrations(version, name, applied_at)
VALUES (1, 'metrology_registry', '1970-01-01T00:00:00Z');
