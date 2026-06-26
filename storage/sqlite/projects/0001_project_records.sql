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

CREATE TABLE projects (
    code TEXT PRIMARY KEY,
    customer_name TEXT NOT NULL,
    stage TEXT NOT NULL CHECK (
        stage IN (
            'quotation',
            'contract_review',
            'test_planning',
            'measuring',
            'technical_review',
            'report_issued',
            'archived'
        )
    ),
    execution_mode TEXT NOT NULL CHECK (
        execution_mode IN ('accredited', 'non_accredited', 'investigation')
    ),
    created_at TEXT NOT NULL,
    archived_at TEXT
);

CREATE TABLE project_audit_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    project_code TEXT NOT NULL REFERENCES projects(code),
    sequence INTEGER NOT NULL,
    actor TEXT NOT NULL,
    action TEXT NOT NULL,
    reason TEXT,
    payload_json TEXT NOT NULL DEFAULT '{}',
    occurred_at TEXT NOT NULL,
    UNIQUE(project_code, sequence)
);

CREATE TABLE contract_review_items (
    project_code TEXT NOT NULL REFERENCES projects(code),
    item TEXT NOT NULL,
    completed INTEGER NOT NULL DEFAULT 0 CHECK (completed IN (0, 1)),
    completed_by TEXT,
    completed_at TEXT,
    comment TEXT,
    PRIMARY KEY (project_code, item)
);

CREATE TABLE campaigns (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    project_code TEXT NOT NULL REFERENCES projects(code),
    name TEXT NOT NULL,
    standard_reference TEXT NOT NULL,
    equipment_under_test TEXT NOT NULL,
    planned_at TEXT,
    started_at TEXT,
    completed_at TEXT
);

CREATE TABLE measurement_runs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    campaign_id INTEGER NOT NULL REFERENCES campaigns(id),
    operator TEXT NOT NULL,
    method_reference TEXT NOT NULL,
    software_version TEXT NOT NULL,
    readiness_report_json TEXT NOT NULL DEFAULT '{}',
    environment_json TEXT NOT NULL DEFAULT '{}',
    started_at TEXT NOT NULL,
    completed_at TEXT
);

CREATE TABLE measurement_run_instruments (
    measurement_run_id INTEGER NOT NULL REFERENCES measurement_runs(id),
    asset_id TEXT NOT NULL,
    calibration_certificate_reference TEXT,
    role TEXT NOT NULL,
    readiness_status TEXT NOT NULL,
    PRIMARY KEY (measurement_run_id, asset_id, role)
);

CREATE TABLE reports (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    project_code TEXT NOT NULL REFERENCES projects(code),
    report_number TEXT NOT NULL,
    revision TEXT NOT NULL,
    status TEXT NOT NULL CHECK (
        status IN ('draft', 'technical_review', 'approved', 'issued', 'void')
    ),
    reviewed_by TEXT,
    approved_by TEXT,
    issued_at TEXT,
    file_reference TEXT,
    checksum TEXT,
    UNIQUE(report_number, revision)
);

INSERT INTO repository_metadata(key, value, updated_at)
VALUES
    ('domain', 'projects', '1970-01-01T00:00:00Z'),
    ('sync_direction', 'bidirectional', '1970-01-01T00:00:00Z');

INSERT INTO schema_migrations(version, name, applied_at)
VALUES (1, 'project_records', '1970-01-01T00:00:00Z');
