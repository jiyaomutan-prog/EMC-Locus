PRAGMA foreign_keys = ON;

CREATE TABLE equipment_category_aliases (
    alias_code TEXT PRIMARY KEY,
    canonical_category_id TEXT NOT NULL REFERENCES equipment_categories(category_id),
    reason TEXT NOT NULL,
    created_at TEXT NOT NULL
);

INSERT INTO equipment_categories(
    category_id, parent_category_id, root_category_id, label, description,
    sort_order, active, system_defined, created_at, updated_at
)
VALUES (
    'frequency_selective_measurement_instruments',
    'measurement_instruments_digitizers',
    'measurement_instruments_digitizers',
    'Recepteurs de mesure et analyseurs de spectre',
    'Instruments selectifs en frequence destines a la mesure et a l''analyse du spectre, incluant les recepteurs de mesure CEM/EMI et les analyseurs de spectre.',
    10, 1, 1, '2026-08-05T00:00:00Z', '2026-08-05T00:00:00Z'
);

INSERT INTO equipment_category_aliases(alias_code, canonical_category_id, reason, created_at)
VALUES
    ('emc_receiver', 'frequency_selective_measurement_instruments', '0.22.2 operator category consolidation', '2026-08-05T00:00:00Z'),
    ('spectrum_analyzer', 'frequency_selective_measurement_instruments', '0.22.2 operator category consolidation', '2026-08-05T00:00:00Z');

INSERT OR IGNORE INTO equipment_category_field_rules(
    category_id, field_id, required, visible, display_group, display_order,
    default_value_json, help_text_override, updated_at
)
SELECT
    'frequency_selective_measurement_instruments', field_id, required, visible,
    display_group, display_order, default_value_json, help_text_override,
    '2026-08-05T00:00:00Z'
FROM equipment_category_field_rules
WHERE category_id IN ('emc_receiver', 'spectrum_analyzer');

UPDATE equipment_categories
SET active = 0, updated_at = '2026-08-05T00:00:00Z'
WHERE category_id IN ('emc_receiver', 'spectrum_analyzer');

INSERT OR REPLACE INTO repository_metadata(key, value, updated_at)
VALUES
    ('equipment_category_alias_contract', 'canonical-category-with-historical-aliases-v1', '2026-08-05T00:00:00Z'),
    ('equipment_catalog_schema', '2026-08-05-v10', '2026-08-05T00:00:00Z');

INSERT INTO schema_migrations(version, name, applied_at)
VALUES (10, 'frequency_selective_category_alias', '2026-08-05T00:00:00Z');
