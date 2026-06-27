PRAGMA foreign_keys = ON;

CREATE TABLE processing_graph_executions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    processing_graph_instance_id INTEGER NOT NULL REFERENCES processing_graph_instances(id),
    execution_reference TEXT NOT NULL,
    executed_by TEXT NOT NULL,
    executed_at TEXT NOT NULL,
    software_version TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('completed', 'failed')),
    output_artifact_count INTEGER NOT NULL CHECK (output_artifact_count >= 0),
    notes TEXT,
    UNIQUE(processing_graph_instance_id, execution_reference)
);

INSERT INTO schema_migrations(version, name, applied_at)
VALUES (5, 'processing_graph_executions', '1970-01-01T00:00:00Z');
