PRAGMA foreign_keys = ON;

CREATE TABLE processing_graph_instances (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    source_dataset_id INTEGER NOT NULL REFERENCES datasets(id),
    graph_reference TEXT NOT NULL,
    graph_revision TEXT NOT NULL,
    operations_json TEXT NOT NULL,
    created_by TEXT NOT NULL,
    created_at TEXT NOT NULL,
    software_version TEXT NOT NULL,
    source_dataset_checksum TEXT,
    graph_checksum TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'active' CHECK (
        status IN ('draft', 'active', 'superseded', 'rejected')
    ),
    UNIQUE(source_dataset_id, graph_reference, graph_revision)
);

INSERT INTO processing_graph_instances (
    source_dataset_id,
    graph_reference,
    graph_revision,
    operations_json,
    created_by,
    created_at,
    software_version,
    source_dataset_checksum,
    graph_checksum,
    status
)
SELECT
    graph.source_dataset_id,
    graph.graph_reference,
    'A',
    graph.operations_json,
    graph.created_by,
    graph.created_at,
    'unknown',
    dataset.checksum,
    graph.checksum,
    'active'
FROM processing_graphs graph
JOIN datasets dataset
    ON dataset.id = graph.source_dataset_id;

INSERT INTO schema_migrations(version, name, applied_at)
VALUES (3, 'processing_graph_instances', '1970-01-01T00:00:00Z');
