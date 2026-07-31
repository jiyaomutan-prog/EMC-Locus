PRAGMA foreign_keys = ON;

ALTER TABLE station_setup_identities
ADD COLUMN current_qualified_revision_id TEXT
    REFERENCES station_setup_revisions(revision_id) DEFERRABLE INITIALLY DEFERRED;

ALTER TABLE station_setup_revisions
ADD COLUMN qualified_at TEXT;

CREATE INDEX station_setup_qualified_revision_idx
ON station_setup_revisions(setup_id, qualified_at)
WHERE qualified_at IS NOT NULL;

INSERT OR REPLACE INTO repository_metadata(key, value, updated_at)
VALUES
    ('station_setup_contract', 'revisioned-material-requirements-v3', '2026-07-31T00:00:00Z'),
    ('station_material_requirement_contract', 'requirement-assignment-readiness-v1', '2026-07-31T00:00:00Z');

INSERT INTO schema_migrations(version, name, applied_at)
VALUES (3, 'station_material_requirements', '2026-07-31T00:00:00Z');
