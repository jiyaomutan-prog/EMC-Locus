PRAGMA foreign_keys = ON;

CREATE TABLE update_install_validation_evidence (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    package_name TEXT NOT NULL,
    package_version TEXT NOT NULL,
    component TEXT NOT NULL,
    installed_version TEXT NOT NULL,
    source TEXT NOT NULL CHECK (source IN ('online_catalog', 'offline_bundle')),
    validation_status TEXT NOT NULL CHECK (validation_status IN ('accepted', 'rejected')),
    signature_required INTEGER NOT NULL CHECK (signature_required IN (0, 1)),
    signature_present INTEGER NOT NULL CHECK (signature_present IN (0, 1)),
    compatibility_minimum_version TEXT NOT NULL,
    compatibility_maximum_version TEXT,
    package_offline_install_allowed INTEGER NOT NULL CHECK (
        package_offline_install_allowed IN (0, 1)
    ),
    policy_offline_install_allowed INTEGER NOT NULL CHECK (
        policy_offline_install_allowed IN (0, 1)
    ),
    measurement_active INTEGER NOT NULL CHECK (measurement_active IN (0, 1)),
    apply_during_measurement_allowed INTEGER NOT NULL CHECK (
        apply_during_measurement_allowed IN (0, 1)
    ),
    reason TEXT,
    validated_by TEXT NOT NULL,
    validated_at TEXT NOT NULL,
    FOREIGN KEY (package_name, package_version, component)
        REFERENCES update_packages(package_name, package_version, component)
);

CREATE INDEX idx_update_install_validation_package
ON update_install_validation_evidence(package_name, package_version, component);

ALTER TABLE update_install_records
ADD COLUMN validation_evidence_id INTEGER
REFERENCES update_install_validation_evidence(id);

INSERT INTO schema_migrations(version, name, applied_at)
VALUES (2, 'install_validation_evidence', '1970-01-01T00:00:00Z');
