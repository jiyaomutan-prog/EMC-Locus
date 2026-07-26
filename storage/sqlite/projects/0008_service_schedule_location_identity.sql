PRAGMA foreign_keys = ON;

ALTER TABLE service_schedule_items
ADD COLUMN laboratory_location_id TEXT;

ALTER TABLE service_schedule_items
ADD COLUMN laboratory_location_label TEXT;

UPDATE service_schedule_items
SET laboratory_location_label = location
WHERE laboratory_location_label IS NULL;

CREATE INDEX service_schedule_location_identity_time_idx
ON service_schedule_items(laboratory_location_id, planned_start_at, planned_end_at)
WHERE laboratory_location_id IS NOT NULL;

INSERT OR REPLACE INTO repository_metadata(key, value, updated_at)
VALUES
    ('service_schedule_location_contract', 'stable-location-identity-v1', '2026-07-16T00:00:00Z');

INSERT INTO schema_migrations(version, name, applied_at)
VALUES (8, 'service_schedule_location_identity', '2026-07-16T00:00:00Z');
