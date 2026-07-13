ALTER TABLE equipment_model_classification_summaries
ADD COLUMN root_category_id TEXT NOT NULL DEFAULT '';

ALTER TABLE equipment_model_classification_summaries
ADD COLUMN is_demo INTEGER NOT NULL DEFAULT 0 CHECK (is_demo IN (0, 1));

CREATE INDEX equipment_model_classification_category_idx
    ON equipment_model_classification_summaries(root_category_id, category_code, status, manufacturer);

CREATE INDEX equipment_model_classification_demo_idx
    ON equipment_model_classification_summaries(is_demo, status, manufacturer);

CREATE TABLE equipment_categories (
    category_id TEXT PRIMARY KEY,
    parent_category_id TEXT REFERENCES equipment_categories(category_id),
    root_category_id TEXT NOT NULL,
    label TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    sort_order INTEGER NOT NULL DEFAULT 0,
    active INTEGER NOT NULL DEFAULT 1 CHECK (active IN (0, 1)),
    system_defined INTEGER NOT NULL DEFAULT 0 CHECK (system_defined IN (0, 1)),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    CHECK (trim(category_id) <> ''),
    CHECK (trim(root_category_id) <> ''),
    CHECK (trim(label) <> '')
);

CREATE INDEX equipment_categories_parent_idx
    ON equipment_categories(parent_category_id, sort_order, label);

CREATE INDEX equipment_categories_root_idx
    ON equipment_categories(root_category_id, active, sort_order, label);

CREATE TABLE equipment_field_definitions (
    field_id TEXT PRIMARY KEY,
    field_code TEXT NOT NULL UNIQUE,
    label TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    data_type TEXT NOT NULL CHECK (data_type IN (
        'short_text',
        'long_text',
        'number',
        'number_with_unit',
        'date',
        'boolean',
        'choice',
        'multi_choice',
        'url',
        'file_reference',
        'object_reference'
    )),
    scope TEXT NOT NULL CHECK (scope IN (
        'equipment_model',
        'physical_asset',
        'metrology_record',
        'station_connection',
        'driver_profile',
        'sensor_definition',
        'daq_channel_profile',
        'acquisition_recipe'
    )),
    required_by_default INTEGER NOT NULL DEFAULT 0 CHECK (required_by_default IN (0, 1)),
    visible_by_default INTEGER NOT NULL DEFAULT 1 CHECK (visible_by_default IN (0, 1)),
    unique_value INTEGER NOT NULL DEFAULT 0 CHECK (unique_value IN (0, 1)),
    unit_quantity TEXT,
    allowed_units_json TEXT NOT NULL DEFAULT '[]',
    option_values_json TEXT NOT NULL DEFAULT '[]',
    validation_regex TEXT,
    default_value_json TEXT,
    display_group TEXT NOT NULL DEFAULT 'Identification',
    display_order INTEGER NOT NULL DEFAULT 0,
    active INTEGER NOT NULL DEFAULT 1 CHECK (active IN (0, 1)),
    system_defined INTEGER NOT NULL DEFAULT 0 CHECK (system_defined IN (0, 1)),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    CHECK (trim(field_id) <> ''),
    CHECK (trim(field_code) <> ''),
    CHECK (trim(label) <> '')
);

CREATE INDEX equipment_field_definitions_scope_idx
    ON equipment_field_definitions(scope, active, display_order, label);

CREATE TABLE equipment_category_field_rules (
    category_id TEXT NOT NULL REFERENCES equipment_categories(category_id) ON DELETE CASCADE,
    field_id TEXT NOT NULL REFERENCES equipment_field_definitions(field_id),
    required INTEGER CHECK (required IN (0, 1)),
    visible INTEGER CHECK (visible IN (0, 1)),
    display_group TEXT,
    display_order INTEGER,
    default_value_json TEXT,
    help_text_override TEXT,
    updated_at TEXT NOT NULL,
    PRIMARY KEY(category_id, field_id)
);

CREATE INDEX equipment_category_field_rules_field_idx
    ON equipment_category_field_rules(field_id);

CREATE TABLE equipment_model_field_values (
    equipment_model_id TEXT NOT NULL REFERENCES equipment_model_identities(equipment_model_id) ON DELETE CASCADE,
    revision_id TEXT NOT NULL REFERENCES equipment_model_revisions(revision_id) ON DELETE CASCADE,
    field_id TEXT NOT NULL REFERENCES equipment_field_definitions(field_id),
    value_json TEXT NOT NULL,
    display_value TEXT NOT NULL,
    PRIMARY KEY(equipment_model_id, revision_id, field_id)
);

CREATE INDEX equipment_model_field_values_field_idx
    ON equipment_model_field_values(field_id, display_value);

CREATE TABLE equipment_model_template_snapshots (
    equipment_model_id TEXT NOT NULL REFERENCES equipment_model_identities(equipment_model_id) ON DELETE CASCADE,
    revision_id TEXT NOT NULL REFERENCES equipment_model_revisions(revision_id) ON DELETE CASCADE,
    category_id TEXT NOT NULL REFERENCES equipment_categories(category_id),
    root_category_id TEXT NOT NULL,
    snapshot_json TEXT NOT NULL,
    snapshot_checksum TEXT NOT NULL,
    created_at TEXT NOT NULL,
    PRIMARY KEY(equipment_model_id, revision_id),
    CHECK (length(snapshot_checksum) = 71 AND substr(snapshot_checksum, 1, 7) = 'sha256:' AND substr(snapshot_checksum, 8) NOT GLOB '*[^0-9a-f]*')
);

INSERT INTO equipment_categories(category_id, parent_category_id, root_category_id, label, description, sort_order, active, system_defined, created_at, updated_at)
VALUES
    ('energy_sources', NULL, 'energy_sources', 'Sources d''énergie', 'Sources électriques, RF, thermiques ou autres utilisées pour alimenter ou solliciter un essai.', 10, 1, 1, '2026-07-13T00:00:00Z', '2026-07-13T00:00:00Z'),
    ('signal_sources', NULL, 'signal_sources', 'Sources de signaux', 'Générateurs et sources produisant des signaux contrôlés.', 20, 1, 1, '2026-07-13T00:00:00Z', '2026-07-13T00:00:00Z'),
    ('rf_equipment', NULL, 'rf_equipment', 'Équipements radiofréquences', 'Éléments RF passifs ou actifs utilisés dans les chaînes CEM.', 30, 1, 1, '2026-07-13T00:00:00Z', '2026-07-13T00:00:00Z'),
    ('sensors_transducers', NULL, 'sensors_transducers', 'Capteurs / transducteurs', 'Capteurs et transducteurs convertissant une grandeur physique en signal mesurable.', 40, 1, 1, '2026-07-13T00:00:00Z', '2026-07-13T00:00:00Z'),
    ('actuators_emitters', NULL, 'actuators_emitters', 'Actionneurs / Émetteurs', 'Émetteurs, actionneurs et équipements appliquant une sollicitation.', 50, 1, 1, '2026-07-13T00:00:00Z', '2026-07-13T00:00:00Z'),
    ('measurement_instruments_digitizers', NULL, 'measurement_instruments_digitizers', 'Instruments de mesure / numériseurs', 'Récepteurs, analyseurs, oscilloscopes, multimètres, wattmètres et DAQ.', 60, 1, 1, '2026-07-13T00:00:00Z', '2026-07-13T00:00:00Z'),
    ('processing_control_systems', NULL, 'processing_control_systems', 'Systèmes de traitement et de contrôle', 'Contrôleurs, logiciels, automates et systèmes de pilotage ou de traitement.', 70, 1, 1, '2026-07-13T00:00:00Z', '2026-07-13T00:00:00Z'),
    ('rf_cable', 'rf_equipment', 'rf_equipment', 'Câbles RF', 'Câbles et cordons RF caractérisés en impédance et perte.', 10, 1, 1, '2026-07-13T00:00:00Z', '2026-07-13T00:00:00Z'),
    ('rf_attenuator', 'rf_equipment', 'rf_equipment', 'Atténuateurs', 'Atténuateurs fixes ou variables pour chaînes RF.', 20, 1, 1, '2026-07-13T00:00:00Z', '2026-07-13T00:00:00Z'),
    ('rf_coupler', 'rf_equipment', 'rf_equipment', 'Coupleurs', 'Coupleurs directionnels, hybrides ou diviseurs de puissance.', 30, 1, 1, '2026-07-13T00:00:00Z', '2026-07-13T00:00:00Z'),
    ('rf_amplifier', 'rf_equipment', 'rf_equipment', 'Amplificateurs', 'Amplificateurs RF de mesure ou d''immunité.', 40, 1, 1, '2026-07-13T00:00:00Z', '2026-07-13T00:00:00Z'),
    ('rf_filter', 'rf_equipment', 'rf_equipment', 'Filtres', 'Filtres RF et réseaux de conditionnement.', 50, 1, 1, '2026-07-13T00:00:00Z', '2026-07-13T00:00:00Z'),
    ('rf_load', 'rf_equipment', 'rf_equipment', 'Charges RF', 'Charges et terminaisons RF.', 60, 1, 1, '2026-07-13T00:00:00Z', '2026-07-13T00:00:00Z'),
    ('receiving_antenna', 'sensors_transducers', 'sensors_transducers', 'Antennes de réception', 'Antennes utilisées comme transducteurs de champ.', 10, 1, 1, '2026-07-13T00:00:00Z', '2026-07-13T00:00:00Z'),
    ('field_probe', 'sensors_transducers', 'sensors_transducers', 'Sondes de champ', 'Sondes de champ électrique ou magnétique.', 20, 1, 1, '2026-07-13T00:00:00Z', '2026-07-13T00:00:00Z'),
    ('current_probe', 'sensors_transducers', 'sensors_transducers', 'Pinces de courant', 'Pinces, sondes et transformateurs de courant.', 30, 1, 1, '2026-07-13T00:00:00Z', '2026-07-13T00:00:00Z'),
    ('accelerometer', 'sensors_transducers', 'sensors_transducers', 'Accéléromètres', 'Capteurs de choc et vibration.', 40, 1, 1, '2026-07-13T00:00:00Z', '2026-07-13T00:00:00Z'),
    ('microphone', 'sensors_transducers', 'sensors_transducers', 'Microphones', 'Capteurs acoustiques et microphones de mesure.', 50, 1, 1, '2026-07-13T00:00:00Z', '2026-07-13T00:00:00Z'),
    ('emc_receiver', 'measurement_instruments_digitizers', 'measurement_instruments_digitizers', 'Récepteurs CEM', 'Récepteurs de mesure CEM.', 10, 1, 1, '2026-07-13T00:00:00Z', '2026-07-13T00:00:00Z'),
    ('spectrum_analyzer', 'measurement_instruments_digitizers', 'measurement_instruments_digitizers', 'Analyseurs de spectre', 'Analyseurs de spectre et signal.', 20, 1, 1, '2026-07-13T00:00:00Z', '2026-07-13T00:00:00Z'),
    ('oscilloscope', 'measurement_instruments_digitizers', 'measurement_instruments_digitizers', 'Oscilloscopes', 'Oscilloscopes numériques ou mixtes.', 30, 1, 1, '2026-07-13T00:00:00Z', '2026-07-13T00:00:00Z'),
    ('rf_power_meter', 'measurement_instruments_digitizers', 'measurement_instruments_digitizers', 'Wattmètres RF', 'Wattmètres et sondes de puissance RF.', 40, 1, 1, '2026-07-13T00:00:00Z', '2026-07-13T00:00:00Z'),
    ('multimeter', 'measurement_instruments_digitizers', 'measurement_instruments_digitizers', 'Multimètres', 'Multimètres et instruments électriques généraux.', 50, 1, 1, '2026-07-13T00:00:00Z', '2026-07-13T00:00:00Z'),
    ('daq', 'measurement_instruments_digitizers', 'measurement_instruments_digitizers', 'DAQ', 'Systèmes d''acquisition et numériseurs multivoies.', 60, 1, 1, '2026-07-13T00:00:00Z', '2026-07-13T00:00:00Z');

INSERT INTO equipment_field_definitions(
    field_id, field_code, label, description, data_type, scope, required_by_default,
    visible_by_default, unique_value, unit_quantity, allowed_units_json, option_values_json,
    validation_regex, default_value_json, display_group, display_order, active,
    system_defined, created_at, updated_at
)
VALUES
    ('field_manufacturer', 'manufacturer', 'Fabricant', 'Fabricant ou marque du modèle.', 'short_text', 'equipment_model', 1, 1, 0, NULL, '[]', '[]', NULL, NULL, 'Identification', 10, 1, 1, '2026-07-13T00:00:00Z', '2026-07-13T00:00:00Z'),
    ('field_model_name', 'model_name', 'Modèle', 'Nom commercial ou référence modèle.', 'short_text', 'equipment_model', 1, 1, 0, NULL, '[]', '[]', NULL, NULL, 'Identification', 20, 1, 1, '2026-07-13T00:00:00Z', '2026-07-13T00:00:00Z'),
    ('field_variant', 'variant', 'Variante', 'Version, option principale ou longueur quand applicable.', 'short_text', 'equipment_model', 0, 1, 0, NULL, '[]', '[]', NULL, NULL, 'Identification', 30, 1, 1, '2026-07-13T00:00:00Z', '2026-07-13T00:00:00Z'),
    ('field_internal_reference', 'internal_reference', 'Référence interne', 'Référence catalogue interne du laboratoire.', 'short_text', 'equipment_model', 0, 1, 1, NULL, '[]', '[]', NULL, NULL, 'Identification', 40, 1, 1, '2026-07-13T00:00:00Z', '2026-07-13T00:00:00Z'),
    ('field_description', 'description', 'Description', 'Description métier courte.', 'long_text', 'equipment_model', 0, 1, 0, NULL, '[]', '[]', NULL, NULL, 'Identification', 50, 1, 1, '2026-07-13T00:00:00Z', '2026-07-13T00:00:00Z'),
    ('field_documentation_url', 'documentation_url', 'Documentation', 'Lien vers datasheet, manuel ou dossier documentaire.', 'url', 'equipment_model', 0, 1, 0, NULL, '[]', '[]', NULL, NULL, 'Documents', 900, 1, 1, '2026-07-13T00:00:00Z', '2026-07-13T00:00:00Z'),
    ('field_supplier', 'supplier', 'Fournisseur', 'Fournisseur privilégié ou fabricant distributeur.', 'short_text', 'equipment_model', 0, 1, 0, NULL, '[]', '[]', NULL, NULL, 'Gestion', 610, 1, 1, '2026-07-13T00:00:00Z', '2026-07-13T00:00:00Z'),
    ('field_owner_laboratory', 'owner_laboratory', 'Laboratoire propriétaire', 'Laboratoire ou service propriétaire.', 'short_text', 'equipment_model', 0, 1, 0, NULL, '[]', '[]', NULL, NULL, 'Gestion', 620, 1, 1, '2026-07-13T00:00:00Z', '2026-07-13T00:00:00Z'),
    ('field_criticality', 'criticality', 'Criticité', 'Criticité métier du modèle.', 'choice', 'equipment_model', 0, 1, 0, NULL, '[]', '["faible","normale","haute","critique"]', NULL, '"normale"', 'Gestion', 630, 1, 1, '2026-07-13T00:00:00Z', '2026-07-13T00:00:00Z'),
    ('field_connector_a', 'connector_a', 'Connecteur A', 'Connecteur ou terminaison côté A.', 'short_text', 'equipment_model', 0, 1, 0, NULL, '[]', '[]', NULL, NULL, 'RF', 110, 1, 1, '2026-07-13T00:00:00Z', '2026-07-13T00:00:00Z'),
    ('field_connector_b', 'connector_b', 'Connecteur B', 'Connecteur ou terminaison côté B.', 'short_text', 'equipment_model', 0, 1, 0, NULL, '[]', '[]', NULL, NULL, 'RF', 120, 1, 1, '2026-07-13T00:00:00Z', '2026-07-13T00:00:00Z'),
    ('field_impedance', 'impedance', 'Impédance', 'Impédance nominale.', 'number_with_unit', 'equipment_model', 0, 1, 0, 'impedance', '["ohm"]', '[]', NULL, '{"value":50,"unit":"ohm"}', 'RF', 130, 1, 1, '2026-07-13T00:00:00Z', '2026-07-13T00:00:00Z'),
    ('field_length', 'length', 'Longueur', 'Longueur utile du câble ou accessoire.', 'number_with_unit', 'equipment_model', 0, 1, 0, 'length', '["m","cm","mm"]', '[]', NULL, NULL, 'RF', 140, 1, 1, '2026-07-13T00:00:00Z', '2026-07-13T00:00:00Z'),
    ('field_frequency_min', 'frequency_min', 'Fréquence min.', 'Fréquence minimale utilisable.', 'number_with_unit', 'equipment_model', 0, 1, 0, 'frequency', '["Hz","kHz","MHz","GHz"]', '[]', NULL, NULL, 'Mesure', 210, 1, 1, '2026-07-13T00:00:00Z', '2026-07-13T00:00:00Z'),
    ('field_frequency_max', 'frequency_max', 'Fréquence max.', 'Fréquence maximale utilisable.', 'number_with_unit', 'equipment_model', 0, 1, 0, 'frequency', '["Hz","kHz","MHz","GHz"]', '[]', NULL, NULL, 'Mesure', 220, 1, 1, '2026-07-13T00:00:00Z', '2026-07-13T00:00:00Z'),
    ('field_typical_loss', 'typical_loss', 'Perte typique', 'Perte ou atténuation typique.', 'number_with_unit', 'equipment_model', 0, 1, 0, 'attenuation', '["dB"]', '[]', NULL, NULL, 'RF', 150, 1, 1, '2026-07-13T00:00:00Z', '2026-07-13T00:00:00Z'),
    ('field_channels_count', 'channels_count', 'Nombre de voies', 'Nombre de voies de mesure ou acquisition.', 'number', 'equipment_model', 0, 1, 0, NULL, '[]', '[]', NULL, NULL, 'Mesure', 230, 1, 1, '2026-07-13T00:00:00Z', '2026-07-13T00:00:00Z'),
    ('field_bandwidth', 'bandwidth', 'Bande passante', 'Bande passante nominale.', 'number_with_unit', 'equipment_model', 0, 1, 0, 'frequency', '["Hz","kHz","MHz","GHz"]', '[]', NULL, NULL, 'Mesure', 240, 1, 1, '2026-07-13T00:00:00Z', '2026-07-13T00:00:00Z'),
    ('field_max_sample_rate', 'max_sample_rate', 'Échantillonnage max.', 'Fréquence maximale d''échantillonnage.', 'number_with_unit', 'equipment_model', 0, 1, 0, 'sample_rate', '["S/s","kS/s","MS/s","GS/s"]', '[]', NULL, NULL, 'Acquisition', 310, 1, 1, '2026-07-13T00:00:00Z', '2026-07-13T00:00:00Z'),
    ('field_input_type', 'input_type', 'Type d''entrée', 'Type principal d''entrée.', 'choice', 'equipment_model', 0, 1, 0, NULL, '[]', '["RF","tension","courant","charge","numérique","autre"]', NULL, NULL, 'Mesure', 250, 1, 1, '2026-07-13T00:00:00Z', '2026-07-13T00:00:00Z'),
    ('field_communication_interfaces', 'communication_interfaces', 'Interfaces de communication', 'Interfaces habituellement disponibles.', 'multi_choice', 'equipment_model', 0, 1, 0, NULL, '[]', '["LAN","USB","GPIB","VISA","RS-232","CAN","EtherCAT","OPC-UA","manuel"]', NULL, NULL, 'Contrôle', 510, 1, 1, '2026-07-13T00:00:00Z', '2026-07-13T00:00:00Z');

INSERT INTO equipment_category_field_rules(category_id, field_id, required, visible, display_group, display_order, default_value_json, help_text_override, updated_at)
SELECT category_id, 'field_manufacturer', 1, 1, 'Identification', 10, NULL, 'Fabricant ou marque visible dans le catalogue.', '2026-07-13T00:00:00Z'
FROM equipment_categories
WHERE parent_category_id IS NULL;

INSERT INTO equipment_category_field_rules(category_id, field_id, required, visible, display_group, display_order, default_value_json, help_text_override, updated_at)
SELECT category_id, 'field_model_name', 1, 1, 'Identification', 20, NULL, 'Nom du modèle tel qu''il apparaît sur la documentation.', '2026-07-13T00:00:00Z'
FROM equipment_categories
WHERE parent_category_id IS NULL;

INSERT INTO equipment_category_field_rules(category_id, field_id, required, visible, display_group, display_order, default_value_json, help_text_override, updated_at)
SELECT category_id, 'field_variant', 0, 1, 'Identification', 30, NULL, NULL, '2026-07-13T00:00:00Z'
FROM equipment_categories
WHERE parent_category_id IS NULL;

INSERT INTO equipment_category_field_rules(category_id, field_id, required, visible, display_group, display_order, default_value_json, help_text_override, updated_at)
SELECT category_id, 'field_internal_reference', 0, 1, 'Identification', 40, NULL, NULL, '2026-07-13T00:00:00Z'
FROM equipment_categories
WHERE parent_category_id IS NULL;

INSERT INTO equipment_category_field_rules(category_id, field_id, required, visible, display_group, display_order, default_value_json, help_text_override, updated_at)
SELECT category_id, 'field_description', 0, 1, 'Identification', 50, NULL, NULL, '2026-07-13T00:00:00Z'
FROM equipment_categories
WHERE parent_category_id IS NULL;

INSERT INTO equipment_category_field_rules(category_id, field_id, required, visible, display_group, display_order, default_value_json, help_text_override, updated_at)
SELECT category_id, 'field_documentation_url', 0, 1, 'Documents', 900, NULL, NULL, '2026-07-13T00:00:00Z'
FROM equipment_categories
WHERE parent_category_id IS NULL;

INSERT INTO equipment_category_field_rules(category_id, field_id, required, visible, display_group, display_order, default_value_json, help_text_override, updated_at)
VALUES
    ('rf_equipment', 'field_supplier', 0, 1, 'Gestion', 610, NULL, NULL, '2026-07-13T00:00:00Z'),
    ('rf_equipment', 'field_criticality', 0, 1, 'Gestion', 630, '"normale"', NULL, '2026-07-13T00:00:00Z'),
    ('rf_cable', 'field_connector_a', 0, 1, 'RF', 110, NULL, 'Connecteur côté générateur ou instrument.', '2026-07-13T00:00:00Z'),
    ('rf_cable', 'field_connector_b', 0, 1, 'RF', 120, NULL, 'Connecteur côté charge, EUT ou autre équipement.', '2026-07-13T00:00:00Z'),
    ('rf_cable', 'field_impedance', 0, 1, 'RF', 130, '{"value":50,"unit":"ohm"}', NULL, '2026-07-13T00:00:00Z'),
    ('rf_cable', 'field_length', 0, 1, 'RF', 140, NULL, NULL, '2026-07-13T00:00:00Z'),
    ('rf_cable', 'field_frequency_min', 0, 1, 'Mesure', 210, NULL, NULL, '2026-07-13T00:00:00Z'),
    ('rf_cable', 'field_frequency_max', 0, 1, 'Mesure', 220, NULL, NULL, '2026-07-13T00:00:00Z'),
    ('rf_cable', 'field_typical_loss', 0, 1, 'RF', 150, NULL, NULL, '2026-07-13T00:00:00Z'),
    ('oscilloscope', 'field_channels_count', 0, 1, 'Mesure', 230, NULL, NULL, '2026-07-13T00:00:00Z'),
    ('oscilloscope', 'field_bandwidth', 0, 1, 'Mesure', 240, NULL, NULL, '2026-07-13T00:00:00Z'),
    ('oscilloscope', 'field_max_sample_rate', 0, 1, 'Acquisition', 310, NULL, NULL, '2026-07-13T00:00:00Z'),
    ('oscilloscope', 'field_input_type', 0, 1, 'Mesure', 250, '"tension"', NULL, '2026-07-13T00:00:00Z'),
    ('oscilloscope', 'field_communication_interfaces', 0, 1, 'Contrôle', 510, '["LAN","USB"]', NULL, '2026-07-13T00:00:00Z'),
    ('daq', 'field_channels_count', 0, 1, 'Acquisition', 230, NULL, NULL, '2026-07-13T00:00:00Z'),
    ('daq', 'field_max_sample_rate', 0, 1, 'Acquisition', 310, NULL, NULL, '2026-07-13T00:00:00Z'),
    ('daq', 'field_input_type', 0, 1, 'Acquisition', 250, NULL, NULL, '2026-07-13T00:00:00Z'),
    ('daq', 'field_communication_interfaces', 0, 1, 'Contrôle', 510, '["LAN","USB"]', NULL, '2026-07-13T00:00:00Z'),
    ('emc_receiver', 'field_frequency_min', 0, 1, 'Mesure', 210, NULL, NULL, '2026-07-13T00:00:00Z'),
    ('emc_receiver', 'field_frequency_max', 0, 1, 'Mesure', 220, NULL, NULL, '2026-07-13T00:00:00Z'),
    ('emc_receiver', 'field_communication_interfaces', 0, 1, 'Contrôle', 510, '["LAN","USB","GPIB","VISA"]', NULL, '2026-07-13T00:00:00Z');

UPDATE repository_metadata
SET value = '2026-07-13-v4', updated_at = '2026-07-13T00:00:00Z'
WHERE key = 'equipment_catalog_schema';

UPDATE repository_metadata
SET value = '0.13.1', updated_at = '2026-07-13T00:00:00Z'
WHERE key = 'equipment_catalog_release';

INSERT INTO schema_migrations(version, name, applied_at)
VALUES (4, 'equipment_taxonomy_field_templates', '2026-07-13T00:00:00Z');
