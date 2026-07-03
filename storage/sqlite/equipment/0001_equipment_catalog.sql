PRAGMA foreign_keys = ON;

CREATE TABLE schema_migrations (
    version INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    applied_at TEXT NOT NULL
);

CREATE TABLE repository_metadata (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE equipment_class_registry (
    class_code TEXT PRIMARY KEY,
    label TEXT NOT NULL,
    driver_profile_allowed INTEGER NOT NULL CHECK (driver_profile_allowed IN (0, 1))
);

INSERT INTO equipment_class_registry(class_code, label, driver_profile_allowed)
VALUES
    ('controllable_instrument', 'Controllable instrument', 1),
    ('daq_device', 'DAQ device', 1),
    ('sensor', 'Sensor', 0),
    ('transducer', 'Transducer', 0),
    ('passive_component', 'Passive component', 0),
    ('switching_device', 'Switching device', 1),
    ('motion_system', 'Motion system', 1),
    ('facility', 'Facility', 0),
    ('software_adapter', 'Software adapter', 1),
    ('manual_equipment', 'Manual equipment', 0);

CREATE TABLE equipment_unit_registry (
    unit_code TEXT PRIMARY KEY,
    quantity_code TEXT NOT NULL,
    scale_to_base REAL,
    logarithmic INTEGER NOT NULL CHECK (logarithmic IN (0, 1))
);

INSERT INTO equipment_unit_registry(unit_code, quantity_code, scale_to_base, logarithmic)
VALUES
    ('Hz', 'frequency', 1.0, 0),
    ('kHz', 'frequency', 1000.0, 0),
    ('MHz', 'frequency', 1000000.0, 0),
    ('GHz', 'frequency', 1000000000.0, 0),
    ('s', 'time', 1.0, 0),
    ('ms', 'time', 0.001, 0),
    ('us', 'time', 0.000001, 0),
    ('ns', 'time', 0.000000001, 0),
    ('V', 'voltage', 1.0, 0),
    ('mV', 'voltage', 0.001, 0),
    ('uV', 'voltage', 0.000001, 0),
    ('A', 'current', 1.0, 0),
    ('mA', 'current', 0.001, 0),
    ('uA', 'current', 0.000001, 0),
    ('W', 'power', 1.0, 0),
    ('mW', 'power', 0.001, 0),
    ('dBm', 'power', NULL, 1),
    ('dBuV', 'voltage', NULL, 1),
    ('dBuV_per_m', 'electric_field', NULL, 1),
    ('dB', 'dimensionless', NULL, 1),
    ('dB_per_m', 'magnetic_field', NULL, 1),
    ('ohm', 'resistance', 1.0, 0),
    ('m', 'distance', 1.0, 0),
    ('cm', 'distance', 0.01, 0),
    ('mm', 'distance', 0.001, 0),
    ('deg', 'angle', 0.017453292519943295, 0),
    ('rad', 'angle', 1.0, 0),
    ('Celsius', 'temperature', 1.0, 0),
    ('percent', 'dimensionless', 0.01, 0),
    ('dimensionless', 'dimensionless', 1.0, 0);

CREATE TABLE equipment_model_identities (
    equipment_model_id TEXT PRIMARY KEY,
    manufacturer TEXT NOT NULL,
    model_name TEXT NOT NULL,
    variant TEXT,
    equipment_class TEXT NOT NULL REFERENCES equipment_class_registry(class_code),
    category_code TEXT NOT NULL,
    current_approved_revision_id TEXT,
    created_by TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    CHECK (trim(equipment_model_id) <> ''),
    CHECK (trim(manufacturer) <> ''),
    CHECK (trim(model_name) <> ''),
    CHECK (trim(category_code) <> '')
);

CREATE INDEX equipment_model_identities_lookup_idx
    ON equipment_model_identities(manufacturer, model_name, variant);

CREATE INDEX equipment_model_identities_class_idx
    ON equipment_model_identities(equipment_class, category_code);

CREATE TABLE equipment_model_revisions (
    revision_id TEXT PRIMARY KEY,
    equipment_model_id TEXT NOT NULL REFERENCES equipment_model_identities(equipment_model_id) ON DELETE CASCADE,
    revision_number INTEGER NOT NULL CHECK (revision_number > 0),
    parent_revision_id TEXT REFERENCES equipment_model_revisions(revision_id),
    status TEXT NOT NULL CHECK (
        status IN ('draft', 'under_review', 'approved', 'superseded', 'suspended', 'retired')
    ),
    definition_schema_version TEXT NOT NULL,
    definition_json TEXT NOT NULL,
    definition_checksum TEXT NOT NULL,
    created_by TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    submitted_at TEXT,
    approved_at TEXT,
    capability_count INTEGER NOT NULL DEFAULT 0 CHECK (capability_count >= 0),
    interface_count INTEGER NOT NULL DEFAULT 0 CHECK (interface_count >= 0),
    signal_port_count INTEGER NOT NULL DEFAULT 0 CHECK (signal_port_count >= 0),
    UNIQUE(equipment_model_id, revision_number),
    CHECK (
        length(definition_checksum) = 71
        AND substr(definition_checksum, 1, 7) = 'sha256:'
        AND substr(definition_checksum, 8) NOT GLOB '*[^0-9A-Fa-f]*'
    ),
    CHECK (
        (status = 'draft' AND submitted_at IS NULL AND approved_at IS NULL)
        OR (status = 'under_review' AND submitted_at IS NOT NULL AND approved_at IS NULL)
        OR (status IN ('approved', 'superseded', 'suspended', 'retired'))
    )
);

CREATE UNIQUE INDEX equipment_model_one_active_draft_idx
    ON equipment_model_revisions(equipment_model_id)
    WHERE status = 'draft';

CREATE INDEX equipment_model_revisions_identity_idx
    ON equipment_model_revisions(equipment_model_id, revision_number);

CREATE INDEX equipment_model_revisions_status_idx
    ON equipment_model_revisions(equipment_model_id, status, updated_at);

CREATE TABLE driver_profile_identities (
    driver_profile_id TEXT PRIMARY KEY,
    equipment_model_id TEXT NOT NULL REFERENCES equipment_model_identities(equipment_model_id) ON DELETE CASCADE,
    label TEXT NOT NULL,
    current_approved_revision_id TEXT,
    created_by TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    CHECK (trim(driver_profile_id) <> ''),
    CHECK (trim(label) <> '')
);

CREATE INDEX driver_profile_identities_model_idx
    ON driver_profile_identities(equipment_model_id, label);

CREATE TABLE driver_profile_revisions (
    revision_id TEXT PRIMARY KEY,
    driver_profile_id TEXT NOT NULL REFERENCES driver_profile_identities(driver_profile_id) ON DELETE CASCADE,
    equipment_model_id TEXT NOT NULL REFERENCES equipment_model_identities(equipment_model_id),
    supported_model_revision_id TEXT NOT NULL REFERENCES equipment_model_revisions(revision_id),
    revision_number INTEGER NOT NULL CHECK (revision_number > 0),
    parent_revision_id TEXT REFERENCES driver_profile_revisions(revision_id),
    status TEXT NOT NULL CHECK (
        status IN ('draft', 'under_review', 'approved', 'superseded', 'suspended', 'retired')
    ),
    definition_schema_version TEXT NOT NULL,
    definition_json TEXT NOT NULL,
    definition_checksum TEXT NOT NULL,
    created_by TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    submitted_at TEXT,
    approved_at TEXT,
    action_count INTEGER NOT NULL DEFAULT 0 CHECK (action_count >= 0),
    UNIQUE(driver_profile_id, revision_number),
    CHECK (
        length(definition_checksum) = 71
        AND substr(definition_checksum, 1, 7) = 'sha256:'
        AND substr(definition_checksum, 8) NOT GLOB '*[^0-9A-Fa-f]*'
    ),
    CHECK (
        (status = 'draft' AND submitted_at IS NULL AND approved_at IS NULL)
        OR (status = 'under_review' AND submitted_at IS NOT NULL AND approved_at IS NULL)
        OR (status IN ('approved', 'superseded', 'suspended', 'retired'))
    )
);

CREATE UNIQUE INDEX driver_profile_one_active_draft_idx
    ON driver_profile_revisions(driver_profile_id)
    WHERE status = 'draft';

CREATE INDEX driver_profile_revisions_identity_idx
    ON driver_profile_revisions(driver_profile_id, revision_number);

CREATE INDEX driver_profile_revisions_model_idx
    ON driver_profile_revisions(equipment_model_id, supported_model_revision_id, status);

CREATE TABLE equipment_audit_events (
    audit_id INTEGER PRIMARY KEY AUTOINCREMENT,
    aggregate_kind TEXT NOT NULL CHECK (aggregate_kind IN ('equipment_model', 'driver_profile')),
    entity_id TEXT NOT NULL,
    revision_id TEXT,
    action TEXT NOT NULL,
    actor TEXT NOT NULL,
    reason TEXT NOT NULL,
    old_revision_id TEXT,
    new_revision_id TEXT,
    old_definition_checksum TEXT,
    new_definition_checksum TEXT,
    operation_id TEXT NOT NULL UNIQUE,
    device_id TEXT NOT NULL,
    correlation_id TEXT NOT NULL,
    payload_json TEXT NOT NULL,
    payload_checksum TEXT NOT NULL,
    occurred_at TEXT NOT NULL,
    CHECK (trim(entity_id) <> ''),
    CHECK (trim(action) <> ''),
    CHECK (trim(actor) <> ''),
    CHECK (trim(reason) <> ''),
    CHECK (
        length(payload_checksum) = 71
        AND substr(payload_checksum, 1, 7) = 'sha256:'
        AND substr(payload_checksum, 8) NOT GLOB '*[^0-9A-Fa-f]*'
    )
);

CREATE INDEX equipment_audit_events_entity_idx
    ON equipment_audit_events(aggregate_kind, entity_id, audit_id);

CREATE INDEX equipment_audit_events_revision_idx
    ON equipment_audit_events(revision_id, audit_id);

INSERT OR REPLACE INTO repository_metadata(key, value, updated_at)
VALUES
    ('equipment_catalog_schema', '2026-07-03-v1', '2026-07-03T00:00:00Z'),
    ('equipment_catalog_release', '0.11.0', '2026-07-03T00:00:00Z');

INSERT INTO schema_migrations(version, name, applied_at)
VALUES (1, 'equipment_catalog', '2026-07-03T00:00:00Z');
