PRAGMA foreign_keys = ON;

ALTER TABLE instruments
ADD COLUMN part_number TEXT;

ALTER TABLE instruments
ADD COLUMN calibration_period_months INTEGER CHECK (
    calibration_period_months IS NULL OR calibration_period_months > 0
);

ALTER TABLE instruments
ADD COLUMN metrology_notes TEXT NOT NULL DEFAULT '';

CREATE TABLE instrument_documents (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    asset_id TEXT NOT NULL REFERENCES instruments(asset_id),
    document_kind TEXT NOT NULL CHECK (
        document_kind IN (
            'certificate',
            'datasheet',
            'transducer_calculation',
            'script',
            'manual',
            'photo',
            'other'
        )
    ),
    title TEXT NOT NULL,
    file_reference TEXT NOT NULL,
    checksum TEXT,
    revision TEXT,
    applies_to_function TEXT,
    uploaded_by TEXT NOT NULL,
    created_at TEXT NOT NULL,
    active INTEGER NOT NULL DEFAULT 1 CHECK (active IN (0, 1)),
    UNIQUE(asset_id, document_kind, title, file_reference)
);

CREATE INDEX instrument_documents_asset_kind_idx
    ON instrument_documents(asset_id, document_kind, active);

INSERT OR REPLACE INTO repository_metadata(key, value, updated_at)
VALUES
    ('instrument_lifecycle_schema', '2026-06-28-v1', '2026-06-28T00:00:00Z');

INSERT INTO schema_migrations(version, name, applied_at)
VALUES (3, 'instrument_documents', '2026-06-28T00:00:00Z');
