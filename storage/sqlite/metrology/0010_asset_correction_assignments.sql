PRAGMA foreign_keys = ON;

ALTER TABLE asset_characterization_events
    ADD COLUMN source_kind TEXT NOT NULL DEFAULT 'characterization'
    CHECK (source_kind IN (
        'calibration', 'characterization', 'verification',
        'manufacturer_certificate', 'internal_measurement'
    ));

ALTER TABLE asset_characterization_events
    ADD COLUMN valid_from TEXT NOT NULL DEFAULT '1970-01-01';

ALTER TABLE asset_characterization_events
    ADD COLUMN environmental_conditions_json TEXT NOT NULL DEFAULT '{}';

ALTER TABLE asset_characterization_events
    ADD COLUMN as_found_json TEXT;

ALTER TABLE asset_characterization_events
    ADD COLUMN as_left_json TEXT;

ALTER TABLE asset_characterization_events
    ADD COLUMN adjustment_performed INTEGER NOT NULL DEFAULT 0
    CHECK (adjustment_performed IN (0, 1));

UPDATE asset_characterization_events
SET valid_from = performed_on
WHERE valid_from = '1970-01-01';

CREATE TABLE asset_correction_assignments (
    assignment_id TEXT PRIMARY KEY,
    asset_id TEXT NOT NULL REFERENCES instruments(asset_id),
    equipment_model_id TEXT NOT NULL,
    equipment_model_revision_id TEXT NOT NULL,
    equipment_model_checksum TEXT NOT NULL,
    signal_path_id TEXT NOT NULL,
    requirement_id TEXT NOT NULL,
    correction_definition_id TEXT NOT NULL,
    correction_revision_id TEXT NOT NULL,
    correction_checksum TEXT NOT NULL,
    source_event_id TEXT NOT NULL REFERENCES asset_characterization_events(characterization_id),
    source_kind TEXT NOT NULL CHECK (source_kind IN (
        'calibration', 'characterization', 'verification',
        'manufacturer_certificate', 'internal_measurement'
    )),
    valid_from TEXT NOT NULL,
    valid_until TEXT,
    status TEXT NOT NULL CHECK (status IN (
        'draft', 'waiting_for_review', 'approved', 'active',
        'expired', 'superseded', 'rejected'
    )),
    conditions_json TEXT NOT NULL DEFAULT '{}',
    assigned_at TEXT NOT NULL,
    assigned_by TEXT NOT NULL,
    submitted_at TEXT,
    approved_at TEXT,
    approved_by TEXT,
    superseded_by TEXT REFERENCES asset_correction_assignments(assignment_id),
    updated_at TEXT NOT NULL,
    revision TEXT NOT NULL,
    CHECK (
        length(equipment_model_checksum) = 71
        AND substr(equipment_model_checksum, 1, 7) = 'sha256:'
        AND equipment_model_checksum = lower(equipment_model_checksum)
        AND substr(equipment_model_checksum, 8) NOT GLOB '*[^0-9a-f]*'
    ),
    CHECK (
        length(correction_checksum) = 71
        AND substr(correction_checksum, 1, 7) = 'sha256:'
        AND correction_checksum = lower(correction_checksum)
        AND substr(correction_checksum, 8) NOT GLOB '*[^0-9a-f]*'
    ),
    CHECK (valid_until IS NULL OR valid_until >= valid_from),
    CHECK (
        (status = 'draft' AND submitted_at IS NULL AND approved_at IS NULL AND approved_by IS NULL)
        OR (status = 'waiting_for_review' AND submitted_at IS NOT NULL AND approved_at IS NULL AND approved_by IS NULL)
        OR (status IN ('approved', 'active', 'expired', 'superseded')
            AND submitted_at IS NOT NULL AND approved_at IS NOT NULL AND approved_by IS NOT NULL)
        OR (status = 'rejected' AND submitted_at IS NOT NULL)
    ),
    CHECK (status <> 'superseded' OR superseded_by IS NOT NULL),
    UNIQUE(asset_id, assignment_id)
);

CREATE INDEX asset_corrections_asset_requirement_idx
    ON asset_correction_assignments(asset_id, signal_path_id, requirement_id, assigned_at DESC);

CREATE INDEX asset_corrections_review_queue_idx
    ON asset_correction_assignments(status, submitted_at, asset_id);

CREATE UNIQUE INDEX asset_corrections_one_active_context_idx
    ON asset_correction_assignments(asset_id, signal_path_id, requirement_id, conditions_json)
    WHERE status = 'active';

INSERT OR REPLACE INTO repository_metadata(key, value, updated_at)
VALUES
    ('metrology_schema_version', '10', '2026-07-14T00:00:00Z'),
    ('asset_correction_assignment_contract', 'reviewed-active-resolution-v1', '2026-07-14T00:00:00Z');

INSERT INTO schema_migrations(version, name, applied_at)
VALUES (10, 'asset_correction_assignments', '2026-07-14T00:00:00Z');
