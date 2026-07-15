PRAGMA foreign_keys = ON;

ALTER TABLE service_schedule_items
ADD COLUMN revision INTEGER NOT NULL DEFAULT 1 CHECK (revision >= 1);

ALTER TABLE service_schedule_items
ADD COLUMN created_by TEXT NOT NULL DEFAULT 'legacy-import';

ALTER TABLE service_schedule_items
ADD COLUMN updated_by TEXT NOT NULL DEFAULT 'legacy-import';

CREATE INDEX service_schedule_operator_time_idx
ON service_schedule_items(assigned_operator, planned_start_at, planned_end_at);

CREATE INDEX service_schedule_location_time_idx
ON service_schedule_items(location, planned_start_at, planned_end_at);

INSERT OR REPLACE INTO repository_metadata(key, value, updated_at)
VALUES
    ('service_schedule_writer', 'emc-locus-agent', '2026-07-15T00:00:00Z'),
    ('service_schedule_schema', '2026-07-15-v2', '2026-07-15T00:00:00Z');

INSERT INTO schema_migrations(version, name, applied_at)
VALUES (6, 'service_schedule_agent_ownership', '2026-07-15T00:00:00Z');
