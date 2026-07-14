PRAGMA foreign_keys = ON;

ALTER TABLE instruments
ADD COLUMN equipment_model_id TEXT;

ALTER TABLE instruments
ADD COLUMN equipment_model_revision_id TEXT;

ALTER TABLE instruments
ADD COLUMN equipment_model_checksum TEXT CHECK (
    equipment_model_checksum IS NULL
    OR (
        length(equipment_model_checksum) = 71
        AND substr(equipment_model_checksum, 1, 7) = 'sha256:'
        AND equipment_model_checksum = lower(equipment_model_checksum)
        AND substr(equipment_model_checksum, 8) NOT GLOB '*[^0-9a-f]*'
    )
);

CREATE INDEX instruments_equipment_model_idx
    ON instruments(equipment_model_id, equipment_model_revision_id);

INSERT OR REPLACE INTO repository_metadata(key, value, updated_at)
VALUES
    ('metrology_schema_version', '8', '2026-07-14T00:00:00Z'),
    ('instrument_catalog_link_contract', 'equipment-model-revision-sha256-v1', '2026-07-14T00:00:00Z');

INSERT INTO schema_migrations(version, name, applied_at)
VALUES (8, 'equipment_model_traceability', '2026-07-14T00:00:00Z');
