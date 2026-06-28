PRAGMA foreign_keys = ON;

CREATE TABLE service_schedule_items (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    item_code TEXT NOT NULL UNIQUE,
    project_code TEXT NOT NULL REFERENCES projects(code),
    title TEXT NOT NULL,
    test_category_code TEXT,
    test_method_code TEXT,
    planned_start_at TEXT NOT NULL,
    planned_end_at TEXT NOT NULL,
    assigned_operator TEXT NOT NULL,
    location TEXT NOT NULL,
    equipment_under_test TEXT NOT NULL,
    status TEXT NOT NULL CHECK (
        status IN ('planned', 'confirmed', 'in_progress', 'completed', 'cancelled')
    ),
    notes TEXT NOT NULL DEFAULT '',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX service_schedule_project_time_idx
    ON service_schedule_items(project_code, planned_start_at, planned_end_at);

CREATE INDEX service_schedule_status_time_idx
    ON service_schedule_items(status, planned_start_at);

INSERT OR REPLACE INTO repository_metadata(key, value, updated_at)
VALUES
    ('service_schedule_schema', '2026-06-28-v1', '2026-06-28T00:00:00Z');

INSERT INTO schema_migrations(version, name, applied_at)
VALUES (2, 'service_schedule', '2026-06-28T00:00:00Z');
