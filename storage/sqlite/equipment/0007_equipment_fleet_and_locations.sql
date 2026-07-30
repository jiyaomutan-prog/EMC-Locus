PRAGMA foreign_keys = ON;

CREATE TABLE laboratory_locations (
    location_id TEXT PRIMARY KEY,
    label TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    status TEXT NOT NULL CHECK (status IN ('active', 'archived')),
    revision INTEGER NOT NULL CHECK (revision > 0),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    CHECK (trim(location_id) <> ''),
    CHECK (trim(label) <> '')
);

CREATE UNIQUE INDEX laboratory_locations_label_unique_idx
ON laboratory_locations(lower(label));

CREATE INDEX laboratory_locations_status_label_idx
ON laboratory_locations(status, label);

CREATE TABLE physical_assets (
    asset_id TEXT PRIMARY KEY,
    inventory_code TEXT NOT NULL,
    serial_number TEXT,
    part_number TEXT,
    equipment_model_id TEXT REFERENCES equipment_model_identities(equipment_model_id),
    equipment_model_revision_id TEXT REFERENCES equipment_model_revisions(revision_id),
    equipment_model_checksum TEXT,
    manufacturer_snapshot TEXT NOT NULL,
    model_name_snapshot TEXT NOT NULL,
    variant_snapshot TEXT,
    category_code_snapshot TEXT NOT NULL,
    category_path_json TEXT NOT NULL,
    laboratory_location_id TEXT REFERENCES laboratory_locations(location_id),
    laboratory_location_label_snapshot TEXT,
    ownership_source TEXT NOT NULL CHECK (
        ownership_source IN (
            'laboratory_owned', 'customer_supplied', 'rented', 'borrowed',
            'external', 'software_license', 'installed_facility'
        )
    ),
    service_state TEXT NOT NULL CHECK (
        service_state IN ('usable', 'restricted', 'in_maintenance', 'out_of_service', 'retired')
    ),
    availability_state TEXT NOT NULL CHECK (
        availability_state IN ('available', 'reserved', 'assigned_to_setup', 'in_test', 'unavailable')
    ),
    service_state_reason TEXT NOT NULL DEFAULT '',
    notes TEXT NOT NULL DEFAULT '',
    revision INTEGER NOT NULL CHECK (revision > 0),
    model_link_state TEXT NOT NULL DEFAULT 'resolved' CHECK (
        model_link_state IN ('resolved', 'migration_review_required')
    ),
    migrated_from_metrology INTEGER NOT NULL DEFAULT 0 CHECK (migrated_from_metrology IN (0, 1)),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    CHECK (trim(asset_id) <> ''),
    CHECK (trim(inventory_code) <> ''),
    CHECK (serial_number IS NULL OR trim(serial_number) <> ''),
    CHECK (part_number IS NULL OR trim(part_number) <> ''),
    CHECK (trim(manufacturer_snapshot) <> ''),
    CHECK (trim(model_name_snapshot) <> ''),
    CHECK (trim(category_code_snapshot) <> ''),
    CHECK (
        (laboratory_location_id IS NULL AND laboratory_location_label_snapshot IS NULL)
        OR (laboratory_location_id IS NOT NULL AND trim(laboratory_location_label_snapshot) <> '')
    ),
    CHECK (
        equipment_model_checksum IS NULL
        OR (
            length(equipment_model_checksum) = 71
            AND substr(equipment_model_checksum, 1, 7) = 'sha256:'
            AND equipment_model_checksum = lower(equipment_model_checksum)
            AND substr(equipment_model_checksum, 8) NOT GLOB '*[^0-9a-f]*'
        )
    ),
    CHECK (
        (model_link_state = 'resolved'
            AND equipment_model_id IS NOT NULL
            AND equipment_model_revision_id IS NOT NULL
            AND equipment_model_checksum IS NOT NULL)
        OR model_link_state = 'migration_review_required'
    ),
    CHECK (
        service_state NOT IN ('in_maintenance', 'out_of_service', 'retired')
        OR availability_state = 'unavailable'
    )
);

CREATE UNIQUE INDEX physical_assets_inventory_code_unique_idx
ON physical_assets(lower(inventory_code));

CREATE INDEX physical_assets_model_idx
ON physical_assets(equipment_model_id, equipment_model_revision_id, inventory_code);

CREATE INDEX physical_assets_category_idx
ON physical_assets(category_code_snapshot, manufacturer_snapshot, model_name_snapshot);

CREATE INDEX physical_assets_location_idx
ON physical_assets(laboratory_location_id, service_state, availability_state);

CREATE INDEX physical_assets_service_idx
ON physical_assets(service_state, availability_state, inventory_code);

CREATE TABLE physical_asset_operations (
    operation_id TEXT PRIMARY KEY,
    asset_id TEXT NOT NULL,
    action TEXT NOT NULL,
    request_checksum TEXT NOT NULL,
    resulting_revision INTEGER NOT NULL CHECK (resulting_revision > 0),
    occurred_at TEXT NOT NULL,
    CHECK (
        length(request_checksum) = 71
        AND substr(request_checksum, 1, 7) = 'sha256:'
        AND request_checksum = lower(request_checksum)
        AND substr(request_checksum, 8) NOT GLOB '*[^0-9a-f]*'
    )
);

CREATE INDEX physical_asset_operations_asset_idx
ON physical_asset_operations(asset_id, occurred_at);

CREATE TABLE physical_asset_audit_events (
    audit_id INTEGER PRIMARY KEY AUTOINCREMENT,
    asset_id TEXT NOT NULL,
    sequence INTEGER NOT NULL CHECK (sequence > 0),
    action TEXT NOT NULL,
    actor TEXT NOT NULL,
    reason TEXT NOT NULL,
    old_revision INTEGER,
    new_revision INTEGER NOT NULL CHECK (new_revision > 0),
    operation_id TEXT NOT NULL UNIQUE REFERENCES physical_asset_operations(operation_id),
    device_id TEXT NOT NULL,
    correlation_id TEXT NOT NULL,
    payload_json TEXT NOT NULL,
    payload_checksum TEXT NOT NULL,
    occurred_at TEXT NOT NULL,
    UNIQUE(asset_id, sequence),
    CHECK (
        length(payload_checksum) = 71
        AND substr(payload_checksum, 1, 7) = 'sha256:'
        AND payload_checksum = lower(payload_checksum)
        AND substr(payload_checksum, 8) NOT GLOB '*[^0-9a-f]*'
    )
);

CREATE INDEX physical_asset_audit_asset_idx
ON physical_asset_audit_events(asset_id, sequence);

CREATE TABLE laboratory_location_operations (
    operation_id TEXT PRIMARY KEY,
    location_id TEXT NOT NULL,
    action TEXT NOT NULL,
    request_checksum TEXT NOT NULL,
    resulting_revision INTEGER NOT NULL CHECK (resulting_revision > 0),
    occurred_at TEXT NOT NULL,
    CHECK (
        length(request_checksum) = 71
        AND substr(request_checksum, 1, 7) = 'sha256:'
        AND request_checksum = lower(request_checksum)
        AND substr(request_checksum, 8) NOT GLOB '*[^0-9a-f]*'
    )
);

CREATE INDEX laboratory_location_operations_location_idx
ON laboratory_location_operations(location_id, occurred_at);

CREATE TABLE laboratory_location_audit_events (
    audit_id INTEGER PRIMARY KEY AUTOINCREMENT,
    location_id TEXT NOT NULL,
    sequence INTEGER NOT NULL CHECK (sequence > 0),
    action TEXT NOT NULL,
    actor TEXT NOT NULL,
    reason TEXT NOT NULL,
    old_revision INTEGER,
    new_revision INTEGER NOT NULL CHECK (new_revision > 0),
    operation_id TEXT NOT NULL UNIQUE REFERENCES laboratory_location_operations(operation_id),
    device_id TEXT NOT NULL,
    correlation_id TEXT NOT NULL,
    payload_json TEXT NOT NULL,
    payload_checksum TEXT NOT NULL,
    occurred_at TEXT NOT NULL,
    UNIQUE(location_id, sequence),
    CHECK (
        length(payload_checksum) = 71
        AND substr(payload_checksum, 1, 7) = 'sha256:'
        AND payload_checksum = lower(payload_checksum)
        AND substr(payload_checksum, 8) NOT GLOB '*[^0-9a-f]*'
    )
);

CREATE INDEX laboratory_location_audit_location_idx
ON laboratory_location_audit_events(location_id, sequence);

CREATE TABLE equipment_cross_domain_migrations (
    migration_id TEXT PRIMARY KEY,
    source_domain TEXT NOT NULL,
    source_schema_version INTEGER NOT NULL CHECK (source_schema_version > 0),
    imported_record_count INTEGER NOT NULL DEFAULT 0 CHECK (imported_record_count >= 0),
    evidence_json TEXT NOT NULL DEFAULT '{}',
    completed_at TEXT NOT NULL
);

DELETE FROM equipment_category_field_rules
WHERE field_id IN ('field_internal_reference', 'field_owner_laboratory');

UPDATE equipment_field_definitions
SET active = 0,
    updated_at = '2026-07-26T00:00:00Z'
WHERE field_id IN ('field_internal_reference', 'field_owner_laboratory');

INSERT OR REPLACE INTO repository_metadata(key, value, updated_at)
VALUES
    ('equipment_catalog_schema', '2026-07-26-v7', '2026-07-26T00:00:00Z'),
    ('equipment_catalog_release', '0.22.0', '2026-07-26T00:00:00Z'),
    ('physical_asset_source_of_truth', 'equipment.sqlite/physical_assets', '2026-07-26T00:00:00Z'),
    ('laboratory_location_source_of_truth', 'equipment.sqlite/laboratory_locations', '2026-07-26T00:00:00Z');

INSERT INTO schema_migrations(version, name, applied_at)
VALUES (7, 'equipment_fleet_and_locations', '2026-07-26T00:00:00Z');
