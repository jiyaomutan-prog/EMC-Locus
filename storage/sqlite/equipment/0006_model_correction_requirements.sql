PRAGMA foreign_keys = ON;

INSERT OR REPLACE INTO repository_metadata(key, value, updated_at)
VALUES
    ('equipment_model_correction_contract', 'requirement-and-classified-model-default-v1', '2026-07-14T00:00:00Z'),
    ('legacy_signal_transformations', 'read-for-controlled-revision-only', '2026-07-14T00:00:00Z');

INSERT INTO schema_migrations(version, name, applied_at)
VALUES (6, 'model_correction_requirements', '2026-07-14T00:00:00Z');
