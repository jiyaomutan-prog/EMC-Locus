PRAGMA foreign_keys = ON;

CREATE UNIQUE INDEX IF NOT EXISTS test_template_one_active_draft_idx
    ON test_template_revisions(template_id)
    WHERE status = 'draft';

INSERT OR REPLACE INTO repository_metadata(key, value, updated_at)
VALUES
    ('test_template_active_draft_policy', 'one_active_draft_per_template', '2026-07-01T00:00:00Z');

INSERT INTO schema_migrations(version, name, applied_at)
VALUES (5, 'single_active_draft', '2026-07-01T00:00:00Z');
