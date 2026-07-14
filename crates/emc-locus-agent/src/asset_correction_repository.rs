use crate::AgentError;
use rusqlite::{params, Connection, OptionalExtension, Transaction};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct StoredAssetCorrectionAssignment {
    pub(crate) assignment_id: String,
    pub(crate) asset_id: String,
    pub(crate) equipment_model_id: String,
    pub(crate) equipment_model_revision_id: String,
    pub(crate) equipment_model_checksum: String,
    pub(crate) signal_path_id: String,
    pub(crate) requirement_id: String,
    pub(crate) correction_definition_id: String,
    pub(crate) correction_revision_id: String,
    pub(crate) correction_checksum: String,
    pub(crate) source_event_id: String,
    pub(crate) source_kind: String,
    pub(crate) valid_from: String,
    pub(crate) valid_until: Option<String>,
    pub(crate) status: String,
    pub(crate) conditions_json: String,
    pub(crate) assigned_at: String,
    pub(crate) assigned_by: String,
    pub(crate) submitted_at: Option<String>,
    pub(crate) approved_at: Option<String>,
    pub(crate) approved_by: Option<String>,
    pub(crate) superseded_by: Option<String>,
    pub(crate) updated_at: String,
    pub(crate) revision: String,
}

pub(crate) struct NewAssetCorrectionAssignment<'a> {
    pub(crate) assignment_id: &'a str,
    pub(crate) asset_id: &'a str,
    pub(crate) equipment_model_id: &'a str,
    pub(crate) equipment_model_revision_id: &'a str,
    pub(crate) equipment_model_checksum: &'a str,
    pub(crate) signal_path_id: &'a str,
    pub(crate) requirement_id: &'a str,
    pub(crate) correction_definition_id: &'a str,
    pub(crate) correction_revision_id: &'a str,
    pub(crate) correction_checksum: &'a str,
    pub(crate) source_event_id: &'a str,
    pub(crate) source_kind: &'a str,
    pub(crate) valid_from: &'a str,
    pub(crate) valid_until: Option<&'a str>,
    pub(crate) conditions_json: &'a str,
    pub(crate) assigned_at: &'a str,
    pub(crate) assigned_by: &'a str,
    pub(crate) revision: &'a str,
}

const ASSIGNMENT_COLUMNS: &str = concat!(
    "assignment_id, asset_id, equipment_model_id, equipment_model_revision_id, ",
    "equipment_model_checksum, signal_path_id, requirement_id, correction_definition_id, ",
    "correction_revision_id, correction_checksum, source_event_id, source_kind, valid_from, ",
    "valid_until, status, conditions_json, assigned_at, assigned_by, submitted_at, approved_at, ",
    "approved_by, superseded_by, updated_at, revision"
);

pub(crate) fn load_asset_correction_assignment(
    connection: &Connection,
    assignment_id: &str,
) -> Result<Option<StoredAssetCorrectionAssignment>, AgentError> {
    connection
        .query_row(
            &format!(
                "SELECT {ASSIGNMENT_COLUMNS} FROM asset_correction_assignments WHERE assignment_id = ?1"
            ),
            params![assignment_id],
            assignment_from_row,
        )
        .optional()
        .map_err(query_error)
}

pub(crate) fn load_asset_correction_assignments(
    connection: &Connection,
    asset_id: &str,
) -> Result<Vec<StoredAssetCorrectionAssignment>, AgentError> {
    let mut statement = connection
        .prepare(&format!(
            "SELECT {ASSIGNMENT_COLUMNS} FROM asset_correction_assignments \
             WHERE asset_id = ?1 ORDER BY assigned_at DESC, assignment_id"
        ))
        .map_err(query_error)?;
    let rows = statement
        .query_map(params![asset_id], assignment_from_row)
        .map_err(query_error)?;
    collect_rows(rows)
}

pub(crate) fn load_waiting_asset_corrections(
    connection: &Connection,
) -> Result<Vec<StoredAssetCorrectionAssignment>, AgentError> {
    let mut statement = connection
        .prepare(&format!(
            "SELECT {ASSIGNMENT_COLUMNS} FROM asset_correction_assignments \
             WHERE status = 'waiting_for_review' ORDER BY submitted_at, asset_id, assignment_id"
        ))
        .map_err(query_error)?;
    let rows = statement
        .query_map([], assignment_from_row)
        .map_err(query_error)?;
    collect_rows(rows)
}

pub(crate) fn insert_asset_correction_assignment(
    transaction: &Transaction<'_>,
    input: NewAssetCorrectionAssignment<'_>,
) -> Result<(), AgentError> {
    transaction
        .execute(
            concat!(
                "INSERT INTO asset_correction_assignments (",
                "assignment_id, asset_id, equipment_model_id, equipment_model_revision_id, ",
                "equipment_model_checksum, signal_path_id, requirement_id, correction_definition_id, ",
                "correction_revision_id, correction_checksum, source_event_id, source_kind, valid_from, ",
                "valid_until, status, conditions_json, assigned_at, assigned_by, updated_at, revision",
                ") VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ",
                "'draft', ?15, ?16, ?17, ?16, ?18)"
            ),
            params![
                input.assignment_id,
                input.asset_id,
                input.equipment_model_id,
                input.equipment_model_revision_id,
                input.equipment_model_checksum,
                input.signal_path_id,
                input.requirement_id,
                input.correction_definition_id,
                input.correction_revision_id,
                input.correction_checksum,
                input.source_event_id,
                input.source_kind,
                input.valid_from,
                input.valid_until,
                input.conditions_json,
                input.assigned_at,
                input.assigned_by,
                input.revision,
            ],
        )
        .map_err(write_error)?;
    Ok(())
}

pub(crate) fn submit_asset_correction_assignment(
    transaction: &Transaction<'_>,
    assignment_id: &str,
    expected_revision: &str,
    timestamp: &str,
    revision: &str,
) -> Result<bool, AgentError> {
    let changed = transaction
        .execute(
            "UPDATE asset_correction_assignments \
             SET status = 'waiting_for_review', submitted_at = ?2, updated_at = ?2, revision = ?3 \
             WHERE assignment_id = ?1 AND status = 'draft' AND revision = ?4",
            params![assignment_id, timestamp, revision, expected_revision],
        )
        .map_err(write_error)?;
    Ok(changed == 1)
}

pub(crate) fn approve_and_activate_asset_correction_assignment(
    transaction: &Transaction<'_>,
    current: &StoredAssetCorrectionAssignment,
    actor: &str,
    timestamp: &str,
    revision: &str,
) -> Result<Vec<StoredAssetCorrectionAssignment>, AgentError> {
    let mut superseded = Vec::new();
    {
        let mut statement = transaction
            .prepare(&format!(
                "SELECT {ASSIGNMENT_COLUMNS} FROM asset_correction_assignments \
                 WHERE asset_id = ?1 AND signal_path_id = ?2 AND requirement_id = ?3 \
                   AND conditions_json = ?4 AND status = 'active' AND assignment_id <> ?5 \
                 ORDER BY assignment_id"
            ))
            .map_err(query_error)?;
        let rows = statement
            .query_map(
                params![
                    current.asset_id,
                    current.signal_path_id,
                    current.requirement_id,
                    current.conditions_json,
                    current.assignment_id,
                ],
                assignment_from_row,
            )
            .map_err(query_error)?;
        for row in rows {
            superseded.push(row.map_err(query_error)?);
        }
    }
    transaction
        .execute(
            "UPDATE asset_correction_assignments \
             SET status = 'superseded', superseded_by = ?5, updated_at = ?6, revision = \
                 'superseded:' || assignment_id || ':' || ?6 \
             WHERE asset_id = ?1 AND signal_path_id = ?2 AND requirement_id = ?3 \
               AND conditions_json = ?4 AND status = 'active' AND assignment_id <> ?5",
            params![
                current.asset_id,
                current.signal_path_id,
                current.requirement_id,
                current.conditions_json,
                current.assignment_id,
                timestamp,
            ],
        )
        .map_err(write_error)?;
    let changed = transaction
        .execute(
            "UPDATE asset_correction_assignments \
             SET status = 'active', approved_at = ?2, approved_by = ?3, updated_at = ?2, revision = ?4 \
             WHERE assignment_id = ?1 AND status = 'waiting_for_review' AND revision = ?5",
            params![current.assignment_id, timestamp, actor, revision, current.revision],
        )
        .map_err(write_error)?;
    if changed != 1 {
        return Err(AgentError::new(
            "asset_correction_transition_conflict",
            "only a correction waiting for review can be approved",
        ));
    }
    Ok(superseded)
}

pub(crate) fn reject_asset_correction_assignment(
    transaction: &Transaction<'_>,
    assignment_id: &str,
    expected_revision: &str,
    timestamp: &str,
    revision: &str,
) -> Result<bool, AgentError> {
    let changed = transaction
        .execute(
            "UPDATE asset_correction_assignments \
             SET status = 'rejected', updated_at = ?2, revision = ?3 \
             WHERE assignment_id = ?1 AND status = 'waiting_for_review' AND revision = ?4",
            params![assignment_id, timestamp, revision, expected_revision],
        )
        .map_err(write_error)?;
    Ok(changed == 1)
}

pub(crate) fn request_asset_correction_changes(
    transaction: &Transaction<'_>,
    assignment_id: &str,
    expected_revision: &str,
    timestamp: &str,
    revision: &str,
) -> Result<bool, AgentError> {
    let changed = transaction
        .execute(
            "UPDATE asset_correction_assignments \
             SET status = 'draft', submitted_at = NULL, updated_at = ?2, revision = ?3 \
             WHERE assignment_id = ?1 AND status = 'waiting_for_review' AND revision = ?4",
            params![assignment_id, timestamp, revision, expected_revision],
        )
        .map_err(write_error)?;
    Ok(changed == 1)
}

fn assignment_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<StoredAssetCorrectionAssignment> {
    Ok(StoredAssetCorrectionAssignment {
        assignment_id: row.get(0)?,
        asset_id: row.get(1)?,
        equipment_model_id: row.get(2)?,
        equipment_model_revision_id: row.get(3)?,
        equipment_model_checksum: row.get(4)?,
        signal_path_id: row.get(5)?,
        requirement_id: row.get(6)?,
        correction_definition_id: row.get(7)?,
        correction_revision_id: row.get(8)?,
        correction_checksum: row.get(9)?,
        source_event_id: row.get(10)?,
        source_kind: row.get(11)?,
        valid_from: row.get(12)?,
        valid_until: row.get(13)?,
        status: row.get(14)?,
        conditions_json: row.get(15)?,
        assigned_at: row.get(16)?,
        assigned_by: row.get(17)?,
        submitted_at: row.get(18)?,
        approved_at: row.get(19)?,
        approved_by: row.get(20)?,
        superseded_by: row.get(21)?,
        updated_at: row.get(22)?,
        revision: row.get(23)?,
    })
}

fn collect_rows(
    rows: rusqlite::MappedRows<
        '_,
        impl FnMut(&rusqlite::Row<'_>) -> rusqlite::Result<StoredAssetCorrectionAssignment>,
    >,
) -> Result<Vec<StoredAssetCorrectionAssignment>, AgentError> {
    let mut values = Vec::new();
    for row in rows {
        values.push(row.map_err(query_error)?);
    }
    Ok(values)
}

fn query_error(error: rusqlite::Error) -> AgentError {
    AgentError::new("asset_correction_query_failed", error.to_string())
}

fn write_error(error: rusqlite::Error) -> AgentError {
    AgentError::new("asset_correction_write_failed", error.to_string())
}
