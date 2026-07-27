PRAGMA foreign_keys = ON;

ALTER TABLE physical_assets
ADD COLUMN administrative_availability TEXT NOT NULL DEFAULT 'available'
CHECK (administrative_availability IN ('available', 'unavailable'));

ALTER TABLE physical_assets
ADD COLUMN administrative_unavailability_reason TEXT NOT NULL DEFAULT '';

ALTER TABLE physical_assets
ADD COLUMN legacy_availability_evidence_json TEXT NOT NULL DEFAULT '{}';

UPDATE physical_assets
SET legacy_availability_evidence_json =
        '{"legacy_availability_state":"' || availability_state || '"}',
    administrative_availability = CASE
        WHEN availability_state = 'unavailable' THEN 'unavailable'
        ELSE 'available'
    END,
    administrative_unavailability_reason = CASE
        WHEN availability_state = 'unavailable' AND trim(service_state_reason) <> ''
            THEN service_state_reason
        WHEN availability_state = 'unavailable'
            THEN 'Indisponibilité héritée du registre 0.22.0 intermédiaire'
        ELSE ''
    END;

UPDATE physical_assets
SET availability_state = administrative_availability;

CREATE TRIGGER physical_assets_administrative_availability_insert_guard
BEFORE INSERT ON physical_assets
WHEN NEW.availability_state NOT IN ('available', 'unavailable')
    OR NEW.availability_state <> NEW.administrative_availability
    OR (NEW.administrative_availability = 'unavailable'
        AND trim(NEW.administrative_unavailability_reason) = '')
    OR (NEW.administrative_availability = 'available'
        AND trim(NEW.administrative_unavailability_reason) <> '')
BEGIN
    SELECT RAISE(ABORT, 'invalid administrative availability');
END;

CREATE TRIGGER physical_assets_administrative_availability_update_guard
BEFORE UPDATE OF availability_state, administrative_availability,
    administrative_unavailability_reason ON physical_assets
WHEN NEW.availability_state NOT IN ('available', 'unavailable')
    OR NEW.availability_state <> NEW.administrative_availability
    OR (NEW.administrative_availability = 'unavailable'
        AND trim(NEW.administrative_unavailability_reason) = '')
    OR (NEW.administrative_availability = 'available'
        AND trim(NEW.administrative_unavailability_reason) <> '')
BEGIN
    SELECT RAISE(ABORT, 'invalid administrative availability');
END;

CREATE INDEX physical_assets_administrative_availability_idx
ON physical_assets(administrative_availability, service_state, inventory_code);

INSERT OR REPLACE INTO repository_metadata(key, value, updated_at)
VALUES
    ('equipment_catalog_schema', '2026-07-27-v9', '2026-07-27T00:00:00Z'),
    ('physical_asset_availability_contract',
        'administrative-state-plus-derived-usage-v1', '2026-07-27T00:00:00Z');

INSERT INTO schema_migrations(version, name, applied_at)
VALUES (9, 'administrative_availability', '2026-07-27T00:00:00Z');
