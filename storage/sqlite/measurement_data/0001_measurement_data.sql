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

CREATE TABLE datasets (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    project_code TEXT NOT NULL,
    campaign_reference TEXT NOT NULL,
    measurement_run_reference TEXT NOT NULL,
    kind TEXT NOT NULL CHECK (
        kind IN ('raw_signal', 'raw_sweep', 'processed_signal', 'result_table', 'report_export')
    ),
    file_reference TEXT NOT NULL,
    checksum TEXT NOT NULL,
    acquired_at TEXT NOT NULL,
    immutable INTEGER NOT NULL DEFAULT 1 CHECK (immutable IN (0, 1))
);

CREATE TABLE signal_channels (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    dataset_id INTEGER NOT NULL REFERENCES datasets(id),
    name TEXT NOT NULL,
    source_kind TEXT NOT NULL,
    unit TEXT NOT NULL,
    sample_rate_hz REAL,
    sample_count INTEGER,
    synchronization_reference TEXT,
    UNIQUE(dataset_id, name)
);

CREATE TABLE processing_graphs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    source_dataset_id INTEGER NOT NULL REFERENCES datasets(id),
    graph_reference TEXT NOT NULL,
    operations_json TEXT NOT NULL,
    created_by TEXT NOT NULL,
    created_at TEXT NOT NULL,
    checksum TEXT NOT NULL,
    UNIQUE(source_dataset_id, graph_reference)
);

CREATE TABLE result_artifacts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    processing_graph_id INTEGER NOT NULL REFERENCES processing_graphs(id),
    artifact_kind TEXT NOT NULL,
    file_reference TEXT NOT NULL,
    checksum TEXT NOT NULL,
    created_at TEXT NOT NULL
);

INSERT INTO repository_metadata(key, value, updated_at)
VALUES
    ('domain', 'measurement_data', '1970-01-01T00:00:00Z'),
    ('sync_direction', 'bidirectional', '1970-01-01T00:00:00Z');

INSERT INTO schema_migrations(version, name, applied_at)
VALUES (1, 'measurement_data', '1970-01-01T00:00:00Z');
