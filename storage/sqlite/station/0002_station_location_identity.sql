PRAGMA foreign_keys = ON;

INSERT OR REPLACE INTO repository_metadata(key, value, updated_at)
VALUES
    ('station_setup_contract', 'revisioned-physical-measurement-setup-v2', '2026-07-16T00:00:00Z'),
    ('station_location_contract', 'stable-location-identity-v1', '2026-07-16T00:00:00Z');

INSERT INTO schema_migrations(version, name, applied_at)
VALUES (2, 'station_location_identity', '2026-07-16T00:00:00Z');
