PRAGMA foreign_keys = ON;

DROP INDEX station_setup_one_draft_idx;

CREATE UNIQUE INDEX station_setup_one_editable_draft_idx
ON station_setup_revisions(setup_id)
WHERE status = 'draft' AND qualified_at IS NULL;

INSERT OR REPLACE INTO repository_metadata(key, value, updated_at)
VALUES (
    'station_setup_editable_revision_contract',
    'one-editable-draft-qualified-history-preserved-v1',
    '2026-07-31T00:00:00Z'
);

INSERT INTO schema_migrations(version, name, applied_at)
VALUES (4, 'station_qualified_revision_derivation', '2026-07-31T00:00:00Z');
