PRAGMA foreign_keys = ON;

INSERT OR IGNORE INTO calibration_events (
    event_id,
    asset_id,
    certificate_reference,
    calibrated_at,
    due_at,
    provider,
    decision,
    as_found_status,
    as_left_status,
    adjustment_performed,
    uncertainty_summary_json,
    traceability_reference,
    comment,
    document_manifest_json,
    recorded_at,
    recorded_by,
    revision
)
SELECT
    'legacy-calibration-' || printf('%04d', id),
    asset_id,
    certificate_reference,
    calibrated_at,
    due_at,
    provider,
    CASE status_at_import
        WHEN 'valid' THEN 'conforming'
        WHEN 'due_soon' THEN 'conforming'
        ELSE 'not_assessed'
    END,
    NULL,
    NULL,
    0,
    uncertainty_json,
    NULL,
    'Migrated from legacy calibration_records.status_at_import=' || status_at_import,
    CASE
        WHEN file_reference IS NOT NULL OR checksum IS NOT NULL THEN json_object(
            'object_id', 'legacy-calibration-' || printf('%04d', id),
            'original_filename', COALESCE(file_reference, certificate_reference),
            'local_reference', file_reference,
            'sha256', CASE
                WHEN checksum IS NULL THEN NULL
                WHEN length(replace(lower(checksum), 'sha256:', '')) = 64
                 AND replace(lower(checksum), 'sha256:', '') GLOB '[0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f]'
                THEN replace(lower(checksum), 'sha256:', '')
                ELSE NULL
            END,
            'revision', 'legacy'
        )
        ELSE NULL
    END,
    created_at,
    'legacy-migration',
    'legacy-calibration-' || printf('%04d', id)
FROM calibration_records;

INSERT OR REPLACE INTO repository_metadata(key, value, updated_at)
VALUES
    ('legacy_calibration_event_backfill', 'calibration_records-to-calibration_events-v1', '2026-06-30T00:00:00Z');

INSERT INTO schema_migrations(version, name, applied_at)
VALUES (7, 'legacy_calibration_events', '2026-06-30T00:00:00Z');
