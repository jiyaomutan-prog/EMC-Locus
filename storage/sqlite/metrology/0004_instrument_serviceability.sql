PRAGMA foreign_keys = ON;

ALTER TABLE instruments
ADD COLUMN serviceability_status TEXT NOT NULL DEFAULT 'usable' CHECK (
    serviceability_status IN ('usable', 'restricted', 'out_of_service', 'retired')
);

ALTER TABLE instruments
ADD COLUMN serviceability_reason TEXT NOT NULL DEFAULT '';

ALTER TABLE instruments
ADD COLUMN serviceability_updated_at TEXT;

ALTER TABLE instruments
ADD COLUMN legacy_availability TEXT;

UPDATE instruments
SET
    legacy_availability = availability,
    serviceability_status = CASE availability
        WHEN 'out_of_service' THEN 'out_of_service'
        ELSE 'usable'
    END,
    serviceability_reason = CASE availability
        WHEN 'reserved' THEN 'Migrated legacy reservation from availability; planning will own reservations.'
        WHEN 'out_of_service' THEN 'Migrated legacy out-of-service availability.'
        ELSE serviceability_reason
    END,
    serviceability_updated_at = updated_at
WHERE legacy_availability IS NULL;

INSERT OR REPLACE INTO repository_metadata(key, value, updated_at)
VALUES
    ('instrument_serviceability_schema', '2026-06-30-v1', '2026-06-30T00:00:00Z'),
    ('instrument_serviceability_reserved_policy', 'legacy reserved availability migrates to usable serviceability', '2026-06-30T00:00:00Z');

INSERT INTO schema_migrations(version, name, applied_at)
VALUES (4, 'instrument_serviceability', '2026-06-30T00:00:00Z');
