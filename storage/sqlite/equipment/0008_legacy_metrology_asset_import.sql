PRAGMA foreign_keys = ON;

ALTER TABLE physical_assets
ADD COLUMN migration_evidence_json TEXT NOT NULL DEFAULT '{}';

INSERT OR REPLACE INTO repository_metadata(key, value, updated_at)
VALUES
    ('equipment_catalog_schema', '2026-07-26-v8', '2026-07-26T00:00:00Z'),
    ('legacy_metrology_asset_import', 'tracked-cross-domain-v1', '2026-07-26T00:00:00Z');

INSERT INTO schema_migrations(version, name, applied_at)
VALUES (8, 'legacy_metrology_asset_import', '2026-07-26T00:00:00Z');
