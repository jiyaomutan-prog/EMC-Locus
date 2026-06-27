PRAGMA foreign_keys = ON;

CREATE TABLE processing_graph_instance_artifacts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    processing_graph_instance_id INTEGER NOT NULL REFERENCES processing_graph_instances(id),
    output_signal_reference TEXT NOT NULL,
    artifact_kind TEXT NOT NULL CHECK (
        artifact_kind IN ('processed_signal', 'result_table')
    ),
    file_reference TEXT NOT NULL,
    checksum TEXT NOT NULL,
    created_at TEXT NOT NULL,
    raw_lineage_json TEXT NOT NULL DEFAULT '[]',
    UNIQUE(
        processing_graph_instance_id,
        output_signal_reference,
        artifact_kind,
        checksum
    )
);

INSERT INTO schema_migrations(version, name, applied_at)
VALUES (4, 'processing_graph_instance_artifacts', '1970-01-01T00:00:00Z');
