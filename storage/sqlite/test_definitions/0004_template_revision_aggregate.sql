PRAGMA foreign_keys = ON;

DROP TABLE IF EXISTS test_template_audit_events;
DROP TABLE IF EXISTS test_templates;

CREATE TABLE test_template_identities (
    template_id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    category_code TEXT NOT NULL REFERENCES test_categories(code),
    current_approved_revision_id TEXT,
    created_by TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    CHECK (trim(template_id) <> ''),
    CHECK (trim(title) <> '')
);

CREATE INDEX test_template_identities_category_idx
    ON test_template_identities(category_code, title);

CREATE TABLE test_template_revisions (
    revision_id TEXT PRIMARY KEY,
    template_id TEXT NOT NULL REFERENCES test_template_identities(template_id) ON DELETE CASCADE,
    revision_number INTEGER NOT NULL CHECK (revision_number > 0),
    parent_revision_id TEXT REFERENCES test_template_revisions(revision_id),
    status TEXT NOT NULL CHECK (
        status IN (
            'draft',
            'under_review',
            'approved',
            'suspended',
            'superseded',
            'retired'
        )
    ),
    definition_schema_version TEXT NOT NULL,
    definition_json TEXT NOT NULL,
    definition_checksum TEXT NOT NULL,
    created_by TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    submitted_at TEXT,
    approved_at TEXT,
    UNIQUE(template_id, revision_number),
    CHECK (trim(revision_id) <> ''),
    CHECK (trim(definition_schema_version) <> ''),
    CHECK (
        length(definition_checksum) = 71
        AND substr(definition_checksum, 1, 7) = 'sha256:'
        AND substr(definition_checksum, 8) NOT GLOB '*[^0-9A-Fa-f]*'
    ),
    CHECK (
        (status = 'draft' AND submitted_at IS NULL AND approved_at IS NULL)
        OR (status = 'under_review' AND submitted_at IS NOT NULL AND approved_at IS NULL)
        OR (status IN ('approved', 'suspended', 'superseded', 'retired'))
    )
);

CREATE INDEX test_template_revisions_template_idx
    ON test_template_revisions(template_id, revision_number);

CREATE INDEX test_template_revisions_status_idx
    ON test_template_revisions(template_id, status, updated_at);

CREATE TABLE test_template_audit_events (
    audit_id INTEGER PRIMARY KEY AUTOINCREMENT,
    template_id TEXT NOT NULL REFERENCES test_template_identities(template_id) ON DELETE CASCADE,
    revision_id TEXT REFERENCES test_template_revisions(revision_id),
    action TEXT NOT NULL,
    actor TEXT NOT NULL,
    reason TEXT NOT NULL,
    old_revision_id TEXT,
    new_revision_id TEXT,
    old_definition_checksum TEXT,
    new_definition_checksum TEXT,
    operation_id TEXT NOT NULL UNIQUE,
    device_id TEXT NOT NULL,
    correlation_id TEXT NOT NULL,
    payload_json TEXT NOT NULL,
    payload_checksum TEXT NOT NULL,
    occurred_at TEXT NOT NULL,
    CHECK (trim(action) <> ''),
    CHECK (trim(actor) <> ''),
    CHECK (trim(reason) <> ''),
    CHECK (
        length(payload_checksum) = 71
        AND substr(payload_checksum, 1, 7) = 'sha256:'
        AND substr(payload_checksum, 8) NOT GLOB '*[^0-9A-Fa-f]*'
    )
);

CREATE INDEX test_template_audit_events_template_idx
    ON test_template_audit_events(template_id, audit_id);

CREATE INDEX test_template_audit_events_revision_idx
    ON test_template_audit_events(revision_id, audit_id);

INSERT OR REPLACE INTO repository_metadata(key, value, updated_at)
VALUES
    ('test_template_contract_revision', '2026-07-01-v2', '2026-07-01T00:00:00Z'),
    ('test_template_0_9_migration_policy', 'reset_0_8_template_rows_no_dual_runtime', '2026-07-01T00:00:00Z');

INSERT INTO schema_migrations(version, name, applied_at)
VALUES (4, 'template_revision_aggregate', '2026-07-01T00:00:00Z');
