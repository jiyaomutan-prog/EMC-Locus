PRAGMA foreign_keys = ON;

CREATE TABLE instrument_observations (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    project_code TEXT NOT NULL,
    campaign_reference TEXT NOT NULL,
    measurement_run_reference TEXT NOT NULL,
    sequence INTEGER NOT NULL CHECK (sequence > 0),
    instrument_code TEXT NOT NULL,
    transport TEXT NOT NULL,
    endpoint TEXT NOT NULL,
    command_message TEXT NOT NULL,
    response_message TEXT NOT NULL,
    success INTEGER NOT NULL CHECK (success IN (0, 1)),
    exchange_attempts INTEGER NOT NULL CHECK (exchange_attempts > 0),
    observed_at TEXT NOT NULL,
    raw_payload_json TEXT NOT NULL DEFAULT '{}',
    UNIQUE(measurement_run_reference, instrument_code, sequence)
);

CREATE INDEX idx_instrument_observations_run
ON instrument_observations(measurement_run_reference, observed_at, id);

CREATE INDEX idx_instrument_observations_instrument
ON instrument_observations(instrument_code, observed_at, id);

INSERT INTO schema_migrations(version, name, applied_at)
VALUES (6, 'instrument_observations', '1970-01-01T00:00:00Z');
