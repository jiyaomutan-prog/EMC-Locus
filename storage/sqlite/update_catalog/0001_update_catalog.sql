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

CREATE TABLE update_packages (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    package_name TEXT NOT NULL,
    package_version TEXT NOT NULL,
    component TEXT NOT NULL,
    compatibility_range TEXT NOT NULL,
    signed_checksum TEXT NOT NULL,
    offline_install_allowed INTEGER NOT NULL DEFAULT 1 CHECK (
        offline_install_allowed IN (0, 1)
    ),
    created_at TEXT NOT NULL,
    UNIQUE(package_name, package_version, component)
);

CREATE TABLE update_install_records (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    package_name TEXT NOT NULL,
    package_version TEXT NOT NULL,
    component TEXT NOT NULL,
    installed_by TEXT NOT NULL,
    installed_at TEXT NOT NULL,
    source TEXT NOT NULL CHECK (source IN ('online_catalog', 'offline_bundle')),
    rollback_reference TEXT
);

INSERT INTO repository_metadata(key, value, updated_at)
VALUES
    ('domain', 'update_catalog', '1970-01-01T00:00:00Z'),
    ('sync_direction', 'pull_from_reference', '1970-01-01T00:00:00Z');

INSERT INTO schema_migrations(version, name, applied_at)
VALUES (1, 'update_catalog', '1970-01-01T00:00:00Z');
