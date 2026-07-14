INSERT INTO equipment_categories(
    category_id, parent_category_id, root_category_id, label, description,
    sort_order, active, system_defined, created_at, updated_at
)
VALUES (
    'general_equipment', NULL, 'general_equipment', 'Général',
    'Champs communs hérités par toutes les familles et catégories d''équipements.',
    0, 1, 1, '2026-07-14T00:00:00Z', '2026-07-14T00:00:00Z'
);

UPDATE equipment_categories
SET parent_category_id = 'general_equipment',
    updated_at = '2026-07-14T00:00:00Z'
WHERE parent_category_id IS NULL
  AND category_id <> 'general_equipment';

DELETE FROM equipment_category_field_rules
WHERE category_id IN (
    'energy_sources',
    'signal_sources',
    'rf_equipment',
    'sensors_transducers',
    'actuators_emitters',
    'measurement_instruments_digitizers',
    'processing_control_systems'
)
AND field_id IN (
    'field_manufacturer',
    'field_model_name',
    'field_variant',
    'field_internal_reference',
    'field_description',
    'field_documentation_url'
);

UPDATE equipment_field_definitions
SET active = 0,
    updated_at = '2026-07-14T00:00:00Z'
WHERE field_id IN ('field_variant', 'field_documentation_url');

INSERT INTO equipment_field_definitions(
    field_id, field_code, label, description, data_type, scope,
    required_by_default, visible_by_default, unique_value, unit_quantity,
    allowed_units_json, option_values_json, validation_regex, default_value_json,
    display_group, display_order, active, system_defined, created_at, updated_at
)
VALUES (
    'field_documentation_file', 'documentation', 'Documentation',
    'Datasheet, manuel ou autre document déposé dans le stockage local.',
    'file_reference', 'equipment_model', 0, 1, 0, NULL,
    '[]', '[]', NULL, NULL, 'Documents', 900, 1, 1,
    '2026-07-14T00:00:00Z', '2026-07-14T00:00:00Z'
);

INSERT INTO equipment_category_field_rules(
    category_id, field_id, required, visible, display_group, display_order,
    default_value_json, help_text_override, updated_at
)
VALUES
    ('general_equipment', 'field_manufacturer', 1, 1, 'Identification', 10, NULL, 'Fabricant ou marque visible dans le catalogue.', '2026-07-14T00:00:00Z'),
    ('general_equipment', 'field_model_name', 1, 1, 'Identification', 20, NULL, 'Nom du modèle tel qu''il apparaît sur la documentation.', '2026-07-14T00:00:00Z'),
    ('general_equipment', 'field_internal_reference', 0, 1, 'Identification', 40, NULL, NULL, '2026-07-14T00:00:00Z'),
    ('general_equipment', 'field_description', 0, 1, 'Identification', 50, NULL, NULL, '2026-07-14T00:00:00Z'),
    ('general_equipment', 'field_documentation_file', 0, 1, 'Documents', 900, NULL, 'Déposez un fichier depuis votre poste.', '2026-07-14T00:00:00Z');

UPDATE repository_metadata
SET value = '2026-07-14-v5', updated_at = '2026-07-14T00:00:00Z'
WHERE key = 'equipment_catalog_schema';

UPDATE repository_metadata
SET value = '0.14.0', updated_at = '2026-07-14T00:00:00Z'
WHERE key = 'equipment_catalog_release';

INSERT INTO schema_migrations(version, name, applied_at)
VALUES (5, 'general_equipment_inheritance', '2026-07-14T00:00:00Z');
