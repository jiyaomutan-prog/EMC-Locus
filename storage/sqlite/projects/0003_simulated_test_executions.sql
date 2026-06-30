PRAGMA foreign_keys = ON;

CREATE TABLE simulated_test_executions (
    attempt_id TEXT PRIMARY KEY,
    project_code TEXT NOT NULL REFERENCES projects(code),
    test_type TEXT NOT NULL CHECK (test_type IN ('simulated_emc')),
    test_method_reference TEXT NOT NULL,
    execution_mode TEXT NOT NULL CHECK (
        execution_mode IN ('accredited', 'non_accredited', 'investigation')
    ),
    operator TEXT NOT NULL,
    checked_on TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('refused', 'completed')),
    readiness_ready INTEGER NOT NULL CHECK (readiness_ready IN (0, 1)),
    readiness_report_json TEXT NOT NULL,
    refusal_json TEXT,
    instrumentation_snapshot_json TEXT NOT NULL,
    simulation_result_json TEXT,
    software_version TEXT NOT NULL,
    started_at TEXT NOT NULL,
    completed_at TEXT NOT NULL,
    revision TEXT NOT NULL,
    CHECK (
        (status = 'refused' AND refusal_json IS NOT NULL AND simulation_result_json IS NULL)
        OR (status = 'completed' AND refusal_json IS NULL AND simulation_result_json IS NOT NULL)
    )
);

CREATE TABLE simulated_test_execution_instruments (
    attempt_id TEXT NOT NULL REFERENCES simulated_test_executions(attempt_id) ON DELETE CASCADE,
    asset_id TEXT NOT NULL,
    role TEXT NOT NULL,
    serviceability_status TEXT,
    calibration_requirement TEXT,
    calibration_status TEXT NOT NULL,
    due_at TEXT,
    blocking INTEGER NOT NULL CHECK (blocking IN (0, 1)),
    reasons_json TEXT NOT NULL DEFAULT '[]',
    instrument_revision TEXT,
    calibration_revision TEXT,
    PRIMARY KEY (attempt_id, asset_id, role)
);

CREATE INDEX idx_simulated_test_executions_project
ON simulated_test_executions(project_code, completed_at, attempt_id);

CREATE INDEX idx_simulated_test_execution_instruments_asset
ON simulated_test_execution_instruments(asset_id, attempt_id);

INSERT INTO schema_migrations(version, name, applied_at)
VALUES (3, 'simulated_test_executions', '2026-06-30T00:00:00Z');
