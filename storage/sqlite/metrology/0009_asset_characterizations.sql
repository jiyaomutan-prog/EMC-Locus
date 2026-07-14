PRAGMA foreign_keys = ON;

CREATE TABLE asset_characterization_events (
    characterization_id TEXT PRIMARY KEY,
    asset_id TEXT NOT NULL REFERENCES instruments(asset_id),
    characterization_kind TEXT NOT NULL CHECK (
        characterization_kind IN ('time_conversion', 'frequency_response')
    ),
    label TEXT NOT NULL,
    performed_on TEXT NOT NULL,
    valid_until TEXT NOT NULL,
    provider TEXT NOT NULL,
    method_reference TEXT NOT NULL,
    decision TEXT NOT NULL CHECK (
        decision IN ('conforming', 'nonconforming', 'indeterminate', 'not_assessed')
    ),
    definition_schema_version TEXT NOT NULL,
    definition_json TEXT NOT NULL,
    definition_checksum TEXT NOT NULL CHECK (
        length(definition_checksum) = 71
        AND substr(definition_checksum, 1, 7) = 'sha256:'
        AND definition_checksum = lower(definition_checksum)
        AND substr(definition_checksum, 8) NOT GLOB '*[^0-9a-f]*'
    ),
    certificate_reference TEXT,
    document_manifest_json TEXT,
    comment TEXT NOT NULL DEFAULT '',
    recorded_at TEXT NOT NULL,
    recorded_by TEXT NOT NULL,
    revision TEXT NOT NULL,
    UNIQUE(asset_id, characterization_id)
);

CREATE INDEX asset_characterizations_asset_date_idx
    ON asset_characterization_events(asset_id, performed_on DESC, recorded_at DESC);

CREATE INDEX asset_characterizations_asset_kind_validity_idx
    ON asset_characterization_events(asset_id, characterization_kind, valid_until DESC);

INSERT OR REPLACE INTO repository_metadata(key, value, updated_at)
VALUES
    ('metrology_schema_version', '9', '2026-07-14T00:00:00Z'),
    ('asset_characterization_contract', 'immutable-canonical-correction-v1', '2026-07-14T00:00:00Z');

INSERT INTO schema_migrations(version, name, applied_at)
VALUES (9, 'asset_characterizations', '2026-07-14T00:00:00Z');
