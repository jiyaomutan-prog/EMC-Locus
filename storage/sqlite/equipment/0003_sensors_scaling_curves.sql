PRAGMA foreign_keys = ON;

ALTER TABLE equipment_unit_registry
    ADD COLUMN compound INTEGER NOT NULL DEFAULT 0 CHECK (compound IN (0, 1));

ALTER TABLE equipment_unit_registry
    ADD COLUMN description TEXT NOT NULL DEFAULT '';

INSERT OR IGNORE INTO equipment_unit_registry(unit_code, quantity_code, scale_to_base, logarithmic, compound, description)
VALUES
    ('g', 'acceleration', 9.80665, 0, 0, 'standard gravity acceleration'),
    ('m_per_s', 'velocity', 1.0, 0, 1, 'metres per second'),
    ('m_per_s2', 'acceleration', 1.0, 0, 1, 'metres per second squared'),
    ('mV_per_g', 'dimensionless', NULL, 0, 1, 'accelerometer sensitivity transfer unit'),
    ('V_per_g', 'dimensionless', NULL, 0, 1, 'accelerometer sensitivity transfer unit'),
    ('dB_SPL', 'sound_pressure', NULL, 1, 0, 'sound pressure level'),
    ('N', 'force', 1.0, 0, 0, 'newton'),
    ('Nm', 'torque', 1.0, 0, 1, 'newton metre'),
    ('strain', 'strain', 1.0, 0, 0, 'dimensionless strain'),
    ('microstrain', 'strain', 0.000001, 0, 0, 'microstrain'),
    ('C', 'electric_charge', 1.0, 0, 0, 'coulomb'),
    ('pC', 'electric_charge', 0.000000000001, 0, 0, 'picocoulomb'),
    ('pC_per_N', 'dimensionless', NULL, 0, 1, 'piezoelectric charge sensitivity transfer unit'),
    ('V_per_A', 'dimensionless', NULL, 0, 1, 'current probe transfer unit'),
    ('mV_per_A', 'dimensionless', NULL, 0, 1, 'current probe sensitivity transfer unit'),
    ('A_per_V', 'dimensionless', NULL, 0, 1, 'inverse current probe scaling transfer unit'),
    ('V_per_V', 'dimensionless', NULL, 0, 1, 'voltage ratio transfer unit'),
    ('dB_ohm', 'dimensionless', NULL, 1, 1, 'transimpedance level'),
    ('dB_per_meter', 'dimensionless', NULL, 1, 1, 'antenna factor or distance-normalized correction'),
    ('dB_per_microampere', 'dimensionless', NULL, 1, 1, 'current-normalized level'),
    ('V_per_meter', 'electric_field', 1.0, 0, 1, 'volts per metre'),
    ('A_per_meter', 'magnetic_field', 1.0, 0, 1, 'amperes per metre'),
    ('Tesla', 'magnetic_flux_density', 1.0, 0, 0, 'tesla'),
    ('uT', 'magnetic_flux_density', 0.000001, 0, 0, 'microtesla'),
    ('rad_per_s', 'angular_velocity', 1.0, 0, 1, 'radians per second'),
    ('percent_RH', 'humidity', 0.01, 0, 1, 'relative humidity percent'),
    ('lux', 'illuminance', 1.0, 0, 0, 'lux'),
    ('kg', 'mass', 1.0, 0, 0, 'kilogram'),
    ('g_mass', 'mass', 0.001, 0, 0, 'gram mass'),
    ('m3_per_s', 'flow_rate', 1.0, 0, 1, 'cubic metres per second');

CREATE TABLE sensor_definition_identities (
    sensor_definition_id TEXT PRIMARY KEY,
    label TEXT NOT NULL,
    summary_kind TEXT NOT NULL,
    current_approved_revision_id TEXT,
    created_by TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    CHECK (trim(sensor_definition_id) <> ''),
    CHECK (trim(label) <> ''),
    CHECK (trim(summary_kind) <> '')
);

CREATE TABLE sensor_definition_revisions (
    revision_id TEXT PRIMARY KEY,
    sensor_definition_id TEXT NOT NULL REFERENCES sensor_definition_identities(sensor_definition_id) ON DELETE CASCADE,
    revision_number INTEGER NOT NULL CHECK (revision_number > 0),
    parent_revision_id TEXT REFERENCES sensor_definition_revisions(revision_id),
    status TEXT NOT NULL CHECK (status IN ('draft', 'under_review', 'approved', 'superseded', 'suspended', 'retired')),
    definition_schema_version TEXT NOT NULL,
    definition_json TEXT NOT NULL,
    definition_checksum TEXT NOT NULL,
    label TEXT NOT NULL,
    summary_kind TEXT NOT NULL,
    created_by TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    submitted_at TEXT,
    approved_at TEXT,
    UNIQUE(sensor_definition_id, revision_number),
    CHECK (length(definition_checksum) = 71 AND substr(definition_checksum, 1, 7) = 'sha256:' AND substr(definition_checksum, 8) NOT GLOB '*[^0-9A-Fa-f]*'),
    CHECK ((status = 'draft' AND submitted_at IS NULL AND approved_at IS NULL) OR (status = 'under_review' AND submitted_at IS NOT NULL AND approved_at IS NULL) OR (status IN ('approved', 'superseded', 'suspended', 'retired')))
);

CREATE UNIQUE INDEX sensor_definition_one_active_draft_idx
    ON sensor_definition_revisions(sensor_definition_id)
    WHERE status = 'draft';

CREATE INDEX sensor_definition_revisions_identity_idx
    ON sensor_definition_revisions(sensor_definition_id, revision_number);

CREATE INDEX sensor_definition_revisions_status_idx
    ON sensor_definition_revisions(sensor_definition_id, status, updated_at);

CREATE TABLE scaling_profile_identities (
    scaling_profile_id TEXT PRIMARY KEY,
    label TEXT NOT NULL,
    summary_kind TEXT NOT NULL,
    current_approved_revision_id TEXT,
    created_by TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    CHECK (trim(scaling_profile_id) <> ''),
    CHECK (trim(label) <> ''),
    CHECK (trim(summary_kind) <> '')
);

CREATE TABLE scaling_profile_revisions (
    revision_id TEXT PRIMARY KEY,
    scaling_profile_id TEXT NOT NULL REFERENCES scaling_profile_identities(scaling_profile_id) ON DELETE CASCADE,
    revision_number INTEGER NOT NULL CHECK (revision_number > 0),
    parent_revision_id TEXT REFERENCES scaling_profile_revisions(revision_id),
    status TEXT NOT NULL CHECK (status IN ('draft', 'under_review', 'approved', 'superseded', 'suspended', 'retired')),
    definition_schema_version TEXT NOT NULL,
    definition_json TEXT NOT NULL,
    definition_checksum TEXT NOT NULL,
    label TEXT NOT NULL,
    summary_kind TEXT NOT NULL,
    created_by TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    submitted_at TEXT,
    approved_at TEXT,
    UNIQUE(scaling_profile_id, revision_number),
    CHECK (length(definition_checksum) = 71 AND substr(definition_checksum, 1, 7) = 'sha256:' AND substr(definition_checksum, 8) NOT GLOB '*[^0-9A-Fa-f]*'),
    CHECK ((status = 'draft' AND submitted_at IS NULL AND approved_at IS NULL) OR (status = 'under_review' AND submitted_at IS NOT NULL AND approved_at IS NULL) OR (status IN ('approved', 'superseded', 'suspended', 'retired')))
);

CREATE UNIQUE INDEX scaling_profile_one_active_draft_idx
    ON scaling_profile_revisions(scaling_profile_id)
    WHERE status = 'draft';

CREATE INDEX scaling_profile_revisions_identity_idx
    ON scaling_profile_revisions(scaling_profile_id, revision_number);

CREATE INDEX scaling_profile_revisions_status_idx
    ON scaling_profile_revisions(scaling_profile_id, status, updated_at);

CREATE TABLE engineering_curve_identities (
    engineering_curve_id TEXT PRIMARY KEY,
    label TEXT NOT NULL,
    summary_kind TEXT NOT NULL,
    current_approved_revision_id TEXT,
    created_by TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    CHECK (trim(engineering_curve_id) <> ''),
    CHECK (trim(label) <> ''),
    CHECK (trim(summary_kind) <> '')
);

CREATE TABLE engineering_curve_revisions (
    revision_id TEXT PRIMARY KEY,
    engineering_curve_id TEXT NOT NULL REFERENCES engineering_curve_identities(engineering_curve_id) ON DELETE CASCADE,
    revision_number INTEGER NOT NULL CHECK (revision_number > 0),
    parent_revision_id TEXT REFERENCES engineering_curve_revisions(revision_id),
    status TEXT NOT NULL CHECK (status IN ('draft', 'under_review', 'approved', 'superseded', 'suspended', 'retired')),
    definition_schema_version TEXT NOT NULL,
    definition_json TEXT NOT NULL,
    definition_checksum TEXT NOT NULL,
    label TEXT NOT NULL,
    summary_kind TEXT NOT NULL,
    created_by TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    submitted_at TEXT,
    approved_at TEXT,
    UNIQUE(engineering_curve_id, revision_number),
    CHECK (length(definition_checksum) = 71 AND substr(definition_checksum, 1, 7) = 'sha256:' AND substr(definition_checksum, 8) NOT GLOB '*[^0-9A-Fa-f]*'),
    CHECK ((status = 'draft' AND submitted_at IS NULL AND approved_at IS NULL) OR (status = 'under_review' AND submitted_at IS NOT NULL AND approved_at IS NULL) OR (status IN ('approved', 'superseded', 'suspended', 'retired')))
);

CREATE UNIQUE INDEX engineering_curve_one_active_draft_idx
    ON engineering_curve_revisions(engineering_curve_id)
    WHERE status = 'draft';

CREATE INDEX engineering_curve_revisions_identity_idx
    ON engineering_curve_revisions(engineering_curve_id, revision_number);

CREATE INDEX engineering_curve_revisions_status_idx
    ON engineering_curve_revisions(engineering_curve_id, status, updated_at);

CREATE TABLE daq_channel_profile_identities (
    daq_channel_profile_id TEXT PRIMARY KEY,
    label TEXT NOT NULL,
    summary_kind TEXT NOT NULL,
    current_approved_revision_id TEXT,
    created_by TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    CHECK (trim(daq_channel_profile_id) <> ''),
    CHECK (trim(label) <> ''),
    CHECK (trim(summary_kind) <> '')
);

CREATE TABLE daq_channel_profile_revisions (
    revision_id TEXT PRIMARY KEY,
    daq_channel_profile_id TEXT NOT NULL REFERENCES daq_channel_profile_identities(daq_channel_profile_id) ON DELETE CASCADE,
    revision_number INTEGER NOT NULL CHECK (revision_number > 0),
    parent_revision_id TEXT REFERENCES daq_channel_profile_revisions(revision_id),
    status TEXT NOT NULL CHECK (status IN ('draft', 'under_review', 'approved', 'superseded', 'suspended', 'retired')),
    definition_schema_version TEXT NOT NULL,
    definition_json TEXT NOT NULL,
    definition_checksum TEXT NOT NULL,
    label TEXT NOT NULL,
    summary_kind TEXT NOT NULL,
    created_by TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    submitted_at TEXT,
    approved_at TEXT,
    UNIQUE(daq_channel_profile_id, revision_number),
    CHECK (length(definition_checksum) = 71 AND substr(definition_checksum, 1, 7) = 'sha256:' AND substr(definition_checksum, 8) NOT GLOB '*[^0-9A-Fa-f]*'),
    CHECK ((status = 'draft' AND submitted_at IS NULL AND approved_at IS NULL) OR (status = 'under_review' AND submitted_at IS NOT NULL AND approved_at IS NULL) OR (status IN ('approved', 'superseded', 'suspended', 'retired')))
);

CREATE UNIQUE INDEX daq_channel_profile_one_active_draft_idx
    ON daq_channel_profile_revisions(daq_channel_profile_id)
    WHERE status = 'draft';

CREATE INDEX daq_channel_profile_revisions_identity_idx
    ON daq_channel_profile_revisions(daq_channel_profile_id, revision_number);

CREATE INDEX daq_channel_profile_revisions_status_idx
    ON daq_channel_profile_revisions(daq_channel_profile_id, status, updated_at);

CREATE TABLE acquisition_channel_recipe_identities (
    acquisition_channel_recipe_id TEXT PRIMARY KEY,
    label TEXT NOT NULL,
    summary_kind TEXT NOT NULL,
    current_approved_revision_id TEXT,
    created_by TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    CHECK (trim(acquisition_channel_recipe_id) <> ''),
    CHECK (trim(label) <> ''),
    CHECK (trim(summary_kind) <> '')
);

CREATE TABLE acquisition_channel_recipe_revisions (
    revision_id TEXT PRIMARY KEY,
    acquisition_channel_recipe_id TEXT NOT NULL REFERENCES acquisition_channel_recipe_identities(acquisition_channel_recipe_id) ON DELETE CASCADE,
    revision_number INTEGER NOT NULL CHECK (revision_number > 0),
    parent_revision_id TEXT REFERENCES acquisition_channel_recipe_revisions(revision_id),
    status TEXT NOT NULL CHECK (status IN ('draft', 'under_review', 'approved', 'superseded', 'suspended', 'retired')),
    definition_schema_version TEXT NOT NULL,
    definition_json TEXT NOT NULL,
    definition_checksum TEXT NOT NULL,
    label TEXT NOT NULL,
    summary_kind TEXT NOT NULL,
    created_by TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    submitted_at TEXT,
    approved_at TEXT,
    UNIQUE(acquisition_channel_recipe_id, revision_number),
    CHECK (length(definition_checksum) = 71 AND substr(definition_checksum, 1, 7) = 'sha256:' AND substr(definition_checksum, 8) NOT GLOB '*[^0-9A-Fa-f]*'),
    CHECK ((status = 'draft' AND submitted_at IS NULL AND approved_at IS NULL) OR (status = 'under_review' AND submitted_at IS NOT NULL AND approved_at IS NULL) OR (status IN ('approved', 'superseded', 'suspended', 'retired')))
);

CREATE UNIQUE INDEX acquisition_channel_recipe_one_active_draft_idx
    ON acquisition_channel_recipe_revisions(acquisition_channel_recipe_id)
    WHERE status = 'draft';

CREATE INDEX acquisition_channel_recipe_revisions_identity_idx
    ON acquisition_channel_recipe_revisions(acquisition_channel_recipe_id, revision_number);

CREATE INDEX acquisition_channel_recipe_revisions_status_idx
    ON acquisition_channel_recipe_revisions(acquisition_channel_recipe_id, status, updated_at);

CREATE TABLE measurement_engineering_audit_events (
    audit_id INTEGER PRIMARY KEY AUTOINCREMENT,
    aggregate_kind TEXT NOT NULL CHECK (aggregate_kind IN ('sensor_definition', 'scaling_profile', 'engineering_curve', 'daq_channel_profile', 'acquisition_channel_recipe')),
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
    CHECK (length(payload_checksum) = 71 AND substr(payload_checksum, 1, 7) = 'sha256:' AND substr(payload_checksum, 8) NOT GLOB '*[^0-9A-Fa-f]*')
);

CREATE INDEX measurement_engineering_audit_entity_idx
    ON measurement_engineering_audit_events(aggregate_kind, entity_id, audit_id);

CREATE INDEX measurement_engineering_audit_revision_idx
    ON measurement_engineering_audit_events(revision_id, audit_id);

UPDATE repository_metadata
SET value = '2026-07-11-v3', updated_at = '2026-07-11T00:00:00Z'
WHERE key = 'equipment_catalog_schema';

UPDATE repository_metadata
SET value = '0.13.0', updated_at = '2026-07-11T00:00:00Z'
WHERE key = 'equipment_catalog_release';

INSERT INTO schema_migrations(version, name, applied_at)
VALUES (3, 'sensors_scaling_curves', '2026-07-11T00:00:00Z');
