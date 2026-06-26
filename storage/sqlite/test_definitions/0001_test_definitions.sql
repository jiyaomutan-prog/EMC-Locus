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

CREATE TABLE standards (
    code TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    edition TEXT NOT NULL,
    issuer TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('active', 'superseded', 'draft')),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE test_methods (
    code TEXT PRIMARY KEY,
    standard_code TEXT REFERENCES standards(code),
    name TEXT NOT NULL,
    family TEXT NOT NULL,
    measurement_axis TEXT NOT NULL CHECK (
        measurement_axis IN (
            'frequency_sweep',
            'time_series',
            'event_triggered',
            'mixed_time_frequency'
        )
    ),
    controlled INTEGER NOT NULL DEFAULT 1 CHECK (controlled IN (0, 1)),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE test_method_revisions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    method_code TEXT NOT NULL REFERENCES test_methods(code),
    revision TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('draft', 'approved', 'retired')),
    parameters_json TEXT NOT NULL DEFAULT '{}',
    acceptance_criteria_json TEXT NOT NULL DEFAULT '{}',
    processing_graph_json TEXT NOT NULL DEFAULT '{}',
    approved_by TEXT,
    approved_at TEXT,
    checksum TEXT,
    UNIQUE(method_code, revision)
);

CREATE TABLE test_steps (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    method_revision_id INTEGER NOT NULL REFERENCES test_method_revisions(id),
    sequence INTEGER NOT NULL,
    name TEXT NOT NULL,
    instruction TEXT NOT NULL,
    expected_evidence TEXT NOT NULL,
    UNIQUE(method_revision_id, sequence)
);

INSERT INTO repository_metadata(key, value, updated_at)
VALUES
    ('domain', 'test_definitions', '1970-01-01T00:00:00Z'),
    ('sync_direction', 'pull_from_reference', '1970-01-01T00:00:00Z');

INSERT INTO schema_migrations(version, name, applied_at)
VALUES (1, 'test_definitions', '1970-01-01T00:00:00Z');
