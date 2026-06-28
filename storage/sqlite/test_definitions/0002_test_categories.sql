PRAGMA foreign_keys = ON;

CREATE TABLE test_categories (
    code TEXT PRIMARY KEY,
    parent_code TEXT REFERENCES test_categories(code),
    label TEXT NOT NULL,
    description TEXT NOT NULL,
    active INTEGER NOT NULL DEFAULT 1 CHECK (active IN (0, 1)),
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX test_categories_parent_idx
    ON test_categories(parent_code, sort_order, label);

ALTER TABLE test_methods
ADD COLUMN category_code TEXT REFERENCES test_categories(code);

INSERT INTO test_categories (
    code,
    parent_code,
    label,
    description,
    active,
    sort_order,
    created_at,
    updated_at
)
VALUES
    (
        'emission',
        NULL,
        'Emission',
        'Essais qui mesurent les perturbations generees par l equipement sous test.',
        1,
        10,
        '2026-06-28T00:00:00Z',
        '2026-06-28T00:00:00Z'
    ),
    (
        'immunity',
        NULL,
        'Immunite',
        'Essais qui verifient le comportement de l equipement sous contrainte electromagnetique.',
        1,
        20,
        '2026-06-28T00:00:00Z',
        '2026-06-28T00:00:00Z'
    ),
    (
        'emission_conducted',
        'emission',
        'Emission conduite',
        'Mesures d emission sur lignes d alimentation, de signal ou de communication.',
        1,
        10,
        '2026-06-28T00:00:00Z',
        '2026-06-28T00:00:00Z'
    ),
    (
        'emission_radiated',
        'emission',
        'Emission rayonnee',
        'Mesures d emission par champ electromagnetique rayonne.',
        1,
        20,
        '2026-06-28T00:00:00Z',
        '2026-06-28T00:00:00Z'
    ),
    (
        'immunity_conducted',
        'immunity',
        'Immunite conduite',
        'Essais d immunite injectes ou couples sur les lignes de l equipement.',
        1,
        10,
        '2026-06-28T00:00:00Z',
        '2026-06-28T00:00:00Z'
    ),
    (
        'immunity_radiated',
        'immunity',
        'Immunite rayonnee',
        'Essais d immunite par champ electromagnetique rayonne.',
        1,
        20,
        '2026-06-28T00:00:00Z',
        '2026-06-28T00:00:00Z'
    ),
    (
        'emission_harmonics_flicker',
        'emission_conducted',
        'Harmoniques et flicker',
        'Mesures d harmoniques, fluctuations de tension et phenomenes basse frequence.',
        1,
        10,
        '2026-06-28T00:00:00Z',
        '2026-06-28T00:00:00Z'
    ),
    (
        'emission_transient_time_domain',
        'emission_conducted',
        'Emission temporelle transitoire',
        'Mesures temporelles telles qu inrush, compteurs d essieux ou signaux impulsionnels.',
        1,
        20,
        '2026-06-28T00:00:00Z',
        '2026-06-28T00:00:00Z'
    ),
    (
        'immunity_esd',
        'immunity_conducted',
        'Decharges electrostatiques',
        'Essais ESD par contact ou dans l air.',
        1,
        10,
        '2026-06-28T00:00:00Z',
        '2026-06-28T00:00:00Z'
    ),
    (
        'immunity_fast_transients',
        'immunity_conducted',
        'Transitoires rapides et salves',
        'Essais EFT/burst, surtensions et perturbations conduites impulsionnelles.',
        1,
        20,
        '2026-06-28T00:00:00Z',
        '2026-06-28T00:00:00Z'
    ),
    (
        'immunity_power_quality',
        'immunity_conducted',
        'Creux, coupures et qualite reseau',
        'Essais de variations d alimentation, creux, interruptions et phenomenes reseau.',
        1,
        30,
        '2026-06-28T00:00:00Z',
        '2026-06-28T00:00:00Z'
    );

INSERT OR REPLACE INTO repository_metadata(key, value, updated_at)
VALUES
    ('test_category_taxonomy_revision', '2026-06-28-v1', '2026-06-28T00:00:00Z'),
    ('test_category_count', '11', '2026-06-28T00:00:00Z');

INSERT INTO schema_migrations(version, name, applied_at)
VALUES (2, 'test_categories', '2026-06-28T00:00:00Z');
