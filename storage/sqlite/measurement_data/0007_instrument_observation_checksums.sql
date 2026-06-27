PRAGMA foreign_keys = ON;

ALTER TABLE instrument_observations
ADD COLUMN observation_checksum TEXT;

CREATE UNIQUE INDEX idx_instrument_observations_checksum
ON instrument_observations(observation_checksum)
WHERE observation_checksum IS NOT NULL;

INSERT INTO schema_migrations(version, name, applied_at)
VALUES (7, 'instrument_observation_checksums', '1970-01-01T00:00:00Z');
