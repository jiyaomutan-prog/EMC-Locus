PRAGMA foreign_keys = ON;

CREATE TABLE test_templates (
    template_id TEXT PRIMARY KEY,
    template_revision TEXT NOT NULL,
    title TEXT NOT NULL,
    description TEXT NOT NULL,
    category_code TEXT NOT NULL REFERENCES test_categories(code),
    measurement_axis TEXT NOT NULL CHECK (
        measurement_axis IN (
            'frequency_sweep',
            'time_series',
            'event_triggered',
            'mixed_time_frequency'
        )
    ),
    method_code TEXT REFERENCES test_methods(code),
    method_revision TEXT,
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
    variables_json TEXT NOT NULL,
    lock_policy_json TEXT NOT NULL,
    instrumentation_chain_json TEXT NOT NULL,
    sequence_json TEXT NOT NULL,
    limits_json TEXT NOT NULL,
    post_processing_json TEXT NOT NULL,
    created_by TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    CHECK (
        (method_code IS NULL AND method_revision IS NULL)
        OR (method_code IS NOT NULL AND method_revision IS NOT NULL)
    )
);

CREATE INDEX test_templates_category_idx
    ON test_templates(category_code, status, title);

CREATE INDEX test_templates_method_idx
    ON test_templates(method_code, method_revision);

CREATE TABLE test_template_audit_events (
    template_id TEXT NOT NULL REFERENCES test_templates(template_id),
    sequence INTEGER NOT NULL,
    actor TEXT NOT NULL,
    action TEXT NOT NULL,
    reason TEXT NOT NULL,
    operation_id TEXT NOT NULL,
    correlation_id TEXT NOT NULL,
    device_id TEXT NOT NULL,
    base_revision TEXT NOT NULL,
    resulting_revision TEXT NOT NULL,
    payload_json TEXT NOT NULL,
    payload_checksum TEXT NOT NULL,
    occurred_at TEXT NOT NULL,
    PRIMARY KEY(template_id, sequence),
    UNIQUE(operation_id)
);

INSERT OR REPLACE INTO repository_metadata(key, value, updated_at)
VALUES
    ('test_template_contract_revision', '2026-06-30-v1', '2026-06-30T00:00:00Z');

INSERT INTO schema_migrations(version, name, applied_at)
VALUES (3, 'test_templates', '2026-06-30T00:00:00Z');
