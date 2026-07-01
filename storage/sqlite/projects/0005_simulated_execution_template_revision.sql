PRAGMA foreign_keys = ON;

ALTER TABLE simulated_test_executions
ADD COLUMN approved_template_id TEXT;

ALTER TABLE simulated_test_executions
ADD COLUMN approved_template_revision_id TEXT;

ALTER TABLE simulated_test_executions
ADD COLUMN approved_template_definition_checksum TEXT;

CREATE INDEX idx_simulated_test_executions_template_revision
ON simulated_test_executions(approved_template_id, approved_template_revision_id);

INSERT INTO schema_migrations(version, name, applied_at)
VALUES (5, 'simulated_execution_template_revision', '2026-07-01T00:00:00Z');
