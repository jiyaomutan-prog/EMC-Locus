PRAGMA foreign_keys = ON;

CREATE TABLE attached_documents (
    document_id TEXT PRIMARY KEY,
    classification TEXT NOT NULL CHECK (
        classification IN (
            'client_document',
            'standard_reference',
            'calibration_certificate',
            'datasheet',
            'worksheet',
            'script',
            'report',
            'photo',
            'drawing',
            'contract',
            'dataset_manifest',
            'other'
        )
    ),
    title TEXT NOT NULL,
    owner_domain TEXT NOT NULL CHECK (
        owner_domain IN (
            'locus_metrology',
            'locus_lab_management',
            'locus_test_station',
            'shared'
        )
    ),
    owner_entity_type TEXT NOT NULL,
    owner_entity_id TEXT NOT NULL,
    storage_backend TEXT NOT NULL CHECK (
        storage_backend IN ('object_store', 'local_path', 'external_reference')
    ),
    storage_uri TEXT NOT NULL,
    original_filename TEXT NOT NULL,
    mime_type TEXT NOT NULL,
    size_bytes INTEGER NOT NULL CHECK (size_bytes >= 0),
    sha256 TEXT NOT NULL CHECK (
        length(sha256) = 64
        AND sha256 NOT GLOB '*[^0-9A-Fa-f]*'
    ),
    revision TEXT NOT NULL,
    applicability TEXT NOT NULL CHECK (
        applicability IN ('draft', 'applicable', 'superseded', 'archival')
    ),
    confidentiality TEXT NOT NULL CHECK (
        confidentiality IN ('internal', 'customer_visible', 'restricted')
    ),
    created_by TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE(
        owner_domain,
        owner_entity_type,
        owner_entity_id,
        classification,
        title,
        revision
    )
);

CREATE INDEX idx_attached_documents_owner
ON attached_documents(owner_domain, owner_entity_type, owner_entity_id, classification);

CREATE TABLE document_audit_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    document_id TEXT NOT NULL REFERENCES attached_documents(document_id),
    sequence INTEGER NOT NULL,
    actor TEXT NOT NULL,
    action TEXT NOT NULL,
    reason TEXT,
    payload_json TEXT NOT NULL DEFAULT '{}',
    occurred_at TEXT NOT NULL,
    UNIQUE(document_id, sequence)
);

INSERT OR REPLACE INTO repository_metadata(key, value, updated_at)
VALUES
    ('attached_documents_schema', '2026-06-30-v1', '1970-01-01T00:00:00Z');

INSERT INTO schema_migrations(version, name, applied_at)
VALUES (4, 'attached_documents', '1970-01-01T00:00:00Z');
