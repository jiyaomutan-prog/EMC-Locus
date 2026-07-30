PRAGMA foreign_keys = OFF;

ALTER TABLE instruments RENAME TO legacy_instruments_0_21_1;

CREATE TABLE metrology_asset_dossiers (
    asset_id TEXT PRIMARY KEY,
    calibration_requirement TEXT NOT NULL CHECK (
        calibration_requirement IN ('required', 'conditional', 'not_required')
    ),
    calibration_period_months INTEGER CHECK (
        calibration_period_months IS NULL OR calibration_period_months > 0
    ),
    calibration_due_warning_days INTEGER NOT NULL DEFAULT 30 CHECK (
        calibration_due_warning_days > 0
    ),
    metrology_notes TEXT NOT NULL DEFAULT '',
    legacy_capabilities_json TEXT NOT NULL DEFAULT '[]',
    revision INTEGER NOT NULL DEFAULT 1 CHECK (revision > 0),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    CHECK (trim(asset_id) <> '')
);

INSERT INTO metrology_asset_dossiers (
    asset_id, calibration_requirement, calibration_period_months,
    calibration_due_warning_days, metrology_notes, legacy_capabilities_json,
    revision, created_at, updated_at
)
SELECT
    asset_id, calibration_requirement, calibration_period_months,
    calibration_due_warning_days, metrology_notes, capabilities_json,
    1, created_at, updated_at
FROM legacy_instruments_0_21_1;

ALTER TABLE calibration_records RENAME TO calibration_records_0_21_1;
CREATE TABLE calibration_records (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    asset_id TEXT NOT NULL REFERENCES metrology_asset_dossiers(asset_id),
    certificate_reference TEXT NOT NULL,
    calibrated_at TEXT NOT NULL,
    due_at TEXT NOT NULL,
    provider TEXT NOT NULL,
    status_at_import TEXT NOT NULL CHECK (
        status_at_import IN ('valid', 'due_soon', 'expired', 'missing', 'not_required')
    ),
    uncertainty_json TEXT NOT NULL DEFAULT '{}',
    file_reference TEXT,
    checksum TEXT,
    created_at TEXT NOT NULL,
    UNIQUE(asset_id, certificate_reference)
);
INSERT INTO calibration_records
SELECT * FROM calibration_records_0_21_1;
DROP TABLE calibration_records_0_21_1;
CREATE INDEX calibration_records_asset_due_idx
    ON calibration_records(asset_id, due_at);

ALTER TABLE instrument_documents RENAME TO instrument_documents_0_21_1;
CREATE TABLE instrument_documents (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    asset_id TEXT NOT NULL REFERENCES metrology_asset_dossiers(asset_id),
    document_kind TEXT NOT NULL CHECK (
        document_kind IN (
            'certificate', 'datasheet', 'transducer_calculation', 'script',
            'manual', 'photo', 'other'
        )
    ),
    title TEXT NOT NULL,
    file_reference TEXT NOT NULL,
    checksum TEXT,
    revision TEXT,
    applies_to_function TEXT,
    uploaded_by TEXT NOT NULL,
    created_at TEXT NOT NULL,
    active INTEGER NOT NULL DEFAULT 1 CHECK (active IN (0, 1)),
    UNIQUE(asset_id, document_kind, title, file_reference)
);
INSERT INTO instrument_documents
SELECT * FROM instrument_documents_0_21_1;
DROP TABLE instrument_documents_0_21_1;
CREATE INDEX instrument_documents_asset_kind_idx
    ON instrument_documents(asset_id, document_kind, active);

ALTER TABLE calibration_events RENAME TO calibration_events_0_21_1;
CREATE TABLE calibration_events (
    event_id TEXT PRIMARY KEY,
    asset_id TEXT NOT NULL REFERENCES metrology_asset_dossiers(asset_id),
    certificate_reference TEXT NOT NULL,
    calibrated_at TEXT NOT NULL,
    due_at TEXT NOT NULL,
    provider TEXT NOT NULL,
    decision TEXT NOT NULL CHECK (
        decision IN ('conforming', 'nonconforming', 'indeterminate', 'not_assessed')
    ),
    as_found_status TEXT CHECK (
        as_found_status IS NULL OR
        as_found_status IN ('conforming', 'nonconforming', 'indeterminate', 'not_assessed')
    ),
    as_left_status TEXT CHECK (
        as_left_status IS NULL OR
        as_left_status IN ('conforming', 'nonconforming', 'indeterminate', 'not_assessed')
    ),
    adjustment_performed INTEGER NOT NULL DEFAULT 0 CHECK (adjustment_performed IN (0, 1)),
    uncertainty_summary_json TEXT NOT NULL DEFAULT '{}',
    traceability_reference TEXT,
    comment TEXT NOT NULL DEFAULT '',
    document_manifest_json TEXT,
    recorded_at TEXT NOT NULL,
    recorded_by TEXT NOT NULL,
    revision TEXT NOT NULL,
    UNIQUE(asset_id, certificate_reference)
);
INSERT INTO calibration_events
SELECT * FROM calibration_events_0_21_1;
DROP TABLE calibration_events_0_21_1;
CREATE INDEX calibration_events_asset_due_idx
    ON calibration_events(asset_id, due_at, calibrated_at);

ALTER TABLE asset_characterization_events RENAME TO asset_characterization_events_0_21_1;
CREATE TABLE asset_characterization_events (
    characterization_id TEXT PRIMARY KEY,
    asset_id TEXT NOT NULL REFERENCES metrology_asset_dossiers(asset_id),
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
    source_kind TEXT NOT NULL DEFAULT 'characterization' CHECK (source_kind IN (
        'calibration', 'characterization', 'verification',
        'manufacturer_certificate', 'internal_measurement'
    )),
    valid_from TEXT NOT NULL,
    environmental_conditions_json TEXT NOT NULL DEFAULT '{}',
    as_found_json TEXT,
    as_left_json TEXT,
    adjustment_performed INTEGER NOT NULL DEFAULT 0 CHECK (adjustment_performed IN (0, 1)),
    UNIQUE(asset_id, characterization_id)
);
INSERT INTO asset_characterization_events
SELECT * FROM asset_characterization_events_0_21_1;
DROP TABLE asset_characterization_events_0_21_1;
CREATE INDEX asset_characterizations_asset_date_idx
    ON asset_characterization_events(asset_id, performed_on DESC, recorded_at DESC);
CREATE INDEX asset_characterizations_asset_kind_validity_idx
    ON asset_characterization_events(asset_id, characterization_kind, valid_until DESC);

ALTER TABLE asset_correction_assignments RENAME TO asset_correction_assignments_0_21_1;
CREATE TABLE asset_correction_assignments (
    assignment_id TEXT PRIMARY KEY,
    asset_id TEXT NOT NULL REFERENCES metrology_asset_dossiers(asset_id),
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
INSERT INTO asset_correction_assignments
SELECT * FROM asset_correction_assignments_0_21_1;
DROP TABLE asset_correction_assignments_0_21_1;
CREATE INDEX asset_corrections_asset_requirement_idx
    ON asset_correction_assignments(asset_id, signal_path_id, requirement_id, assigned_at DESC);
CREATE INDEX asset_corrections_review_queue_idx
    ON asset_correction_assignments(status, submitted_at, asset_id);
CREATE UNIQUE INDEX asset_corrections_one_active_context_idx
    ON asset_correction_assignments(asset_id, signal_path_id, requirement_id, conditions_json)
    WHERE status = 'active';

CREATE TRIGGER legacy_instruments_0_21_1_read_only_insert
BEFORE INSERT ON legacy_instruments_0_21_1
BEGIN
    SELECT RAISE(ABORT, 'legacy instrument identity is read-only');
END;

CREATE TRIGGER legacy_instruments_0_21_1_read_only_update
BEFORE UPDATE ON legacy_instruments_0_21_1
BEGIN
    SELECT RAISE(ABORT, 'legacy instrument identity is read-only');
END;

CREATE TRIGGER legacy_instruments_0_21_1_read_only_delete
BEFORE DELETE ON legacy_instruments_0_21_1
BEGIN
    SELECT RAISE(ABORT, 'legacy instrument identity is read-only');
END;

INSERT OR REPLACE INTO repository_metadata(key, value, updated_at)
VALUES
    ('metrology_schema_version', '11', '2026-07-26T00:00:00Z'),
    ('physical_asset_identity_owner', 'equipment.sqlite/physical_assets', '2026-07-26T00:00:00Z'),
    ('legacy_instrument_archive', 'legacy_instruments_0_21_1/read-only', '2026-07-26T00:00:00Z');

INSERT INTO schema_migrations(version, name, applied_at)
VALUES (11, 'physical_asset_boundary', '2026-07-26T00:00:00Z');

PRAGMA foreign_keys = ON;
