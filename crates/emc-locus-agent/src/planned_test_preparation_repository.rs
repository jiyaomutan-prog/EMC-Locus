use crate::{project_repository::table_exists, AgentError};
use rusqlite::{params, Connection, OptionalExtension, Row, Transaction};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct StoredPlannedTestPreparationIdentity {
    pub(crate) project_code: String,
    pub(crate) schedule_item_code: String,
    pub(crate) current_revision_id: Option<String>,
    pub(crate) created_by: String,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct StoredPlannedTestPreparationRevision {
    pub(crate) revision_id: String,
    pub(crate) project_code: String,
    pub(crate) schedule_item_code: String,
    pub(crate) revision_number: u32,
    pub(crate) parent_revision_id: Option<String>,
    pub(crate) schedule_revision: u64,
    pub(crate) method_template_id: String,
    pub(crate) method_revision_id: String,
    pub(crate) method_definition_checksum: String,
    pub(crate) station_setup_id: String,
    pub(crate) station_setup_revision_id: String,
    pub(crate) station_setup_definition_checksum: String,
    pub(crate) verdict_state: String,
    pub(crate) definition_schema_version: String,
    pub(crate) definition_json: String,
    pub(crate) definition_checksum: String,
    pub(crate) operation_id: String,
    pub(crate) request_checksum: String,
    pub(crate) actor: String,
    pub(crate) reason: String,
    pub(crate) device_id: String,
    pub(crate) correlation_id: String,
    pub(crate) created_at: String,
}

pub(crate) struct NewPlannedTestPreparationIdentity<'a> {
    pub(crate) project_code: &'a str,
    pub(crate) schedule_item_code: &'a str,
    pub(crate) created_by: &'a str,
    pub(crate) timestamp: &'a str,
}

pub(crate) struct NewPlannedTestPreparationRevision<'a> {
    pub(crate) revision_id: &'a str,
    pub(crate) project_code: &'a str,
    pub(crate) schedule_item_code: &'a str,
    pub(crate) revision_number: u32,
    pub(crate) parent_revision_id: Option<&'a str>,
    pub(crate) schedule_revision: u64,
    pub(crate) method_template_id: &'a str,
    pub(crate) method_revision_id: &'a str,
    pub(crate) method_definition_checksum: &'a str,
    pub(crate) station_setup_id: &'a str,
    pub(crate) station_setup_revision_id: &'a str,
    pub(crate) station_setup_definition_checksum: &'a str,
    pub(crate) verdict_state: &'a str,
    pub(crate) definition_schema_version: &'a str,
    pub(crate) definition_json: &'a str,
    pub(crate) definition_checksum: &'a str,
    pub(crate) operation_id: &'a str,
    pub(crate) request_checksum: &'a str,
    pub(crate) actor: &'a str,
    pub(crate) reason: &'a str,
    pub(crate) device_id: &'a str,
    pub(crate) correlation_id: &'a str,
    pub(crate) timestamp: &'a str,
}

pub(crate) fn ensure_planned_test_preparation_tables(
    connection: &Connection,
) -> Result<(), AgentError> {
    for table in [
        "planned_test_preparation_identities",
        "planned_test_preparation_revisions",
    ] {
        if !table_exists(connection, "main", table)? {
            return Err(AgentError::new(
                "storage_migration_required",
                format!("missing required planned test preparation table {table}"),
            ));
        }
    }
    Ok(())
}

pub(crate) fn load_planned_test_preparation_identity(
    connection: &Connection,
    project_code: &str,
    schedule_item_code: &str,
) -> Result<Option<StoredPlannedTestPreparationIdentity>, AgentError> {
    connection
        .query_row(
            concat!(
                "SELECT project_code, schedule_item_code, current_revision_id, created_by, ",
                "created_at, updated_at FROM planned_test_preparation_identities ",
                "WHERE project_code = ?1 AND schedule_item_code = ?2"
            ),
            params![project_code, schedule_item_code],
            stored_identity_from_row,
        )
        .optional()
        .map_err(|error| {
            AgentError::new("planned_test_preparation_query_failed", error.to_string())
        })
}

pub(crate) fn load_current_planned_test_preparation_revision(
    connection: &Connection,
    project_code: &str,
    schedule_item_code: &str,
) -> Result<Option<StoredPlannedTestPreparationRevision>, AgentError> {
    connection
        .query_row(
            &format!(
                concat!(
                    "{} JOIN planned_test_preparation_identities i ",
                    "ON i.current_revision_id = r.revision_id ",
                    "WHERE i.project_code = ?1 AND i.schedule_item_code = ?2"
                ),
                revision_select("r")
            ),
            params![project_code, schedule_item_code],
            stored_revision_from_row,
        )
        .optional()
        .map_err(|error| {
            AgentError::new("planned_test_preparation_query_failed", error.to_string())
        })
}

pub(crate) fn load_planned_test_preparation_revision(
    connection: &Connection,
    revision_id: &str,
) -> Result<Option<StoredPlannedTestPreparationRevision>, AgentError> {
    connection
        .query_row(
            &format!("{} WHERE r.revision_id = ?1", revision_select("r")),
            params![revision_id],
            stored_revision_from_row,
        )
        .optional()
        .map_err(|error| {
            AgentError::new("planned_test_preparation_query_failed", error.to_string())
        })
}

pub(crate) fn load_planned_test_preparation_operation(
    connection: &Connection,
    operation_id: &str,
) -> Result<Option<StoredPlannedTestPreparationRevision>, AgentError> {
    connection
        .query_row(
            &format!("{} WHERE r.operation_id = ?1", revision_select("r")),
            params![operation_id],
            stored_revision_from_row,
        )
        .optional()
        .map_err(|error| {
            AgentError::new("planned_test_preparation_query_failed", error.to_string())
        })
}

pub(crate) fn list_planned_test_preparation_revisions(
    connection: &Connection,
    project_code: &str,
    schedule_item_code: &str,
) -> Result<Vec<StoredPlannedTestPreparationRevision>, AgentError> {
    let mut statement = connection
        .prepare(&format!(
            concat!(
                "{} WHERE r.project_code = ?1 AND r.schedule_item_code = ?2 ",
                "ORDER BY r.revision_number DESC"
            ),
            revision_select("r")
        ))
        .map_err(|error| {
            AgentError::new("planned_test_preparation_query_failed", error.to_string())
        })?;
    let rows = statement
        .query_map(
            params![project_code, schedule_item_code],
            stored_revision_from_row,
        )
        .map_err(|error| {
            AgentError::new("planned_test_preparation_query_failed", error.to_string())
        })?;
    let mut revisions = Vec::new();
    for row in rows {
        revisions.push(row.map_err(|error| {
            AgentError::new("planned_test_preparation_query_failed", error.to_string())
        })?);
    }
    Ok(revisions)
}

pub(crate) fn next_planned_test_preparation_revision_number(
    connection: &Connection,
    project_code: &str,
    schedule_item_code: &str,
) -> Result<u32, AgentError> {
    connection
        .query_row(
            concat!(
                "SELECT COALESCE(MAX(revision_number), 0) + 1 ",
                "FROM planned_test_preparation_revisions ",
                "WHERE project_code = ?1 AND schedule_item_code = ?2"
            ),
            params![project_code, schedule_item_code],
            |row| row.get(0),
        )
        .map_err(|error| {
            AgentError::new("planned_test_preparation_query_failed", error.to_string())
        })
}

pub(crate) fn insert_planned_test_preparation_identity_if_missing(
    transaction: &Transaction<'_>,
    input: NewPlannedTestPreparationIdentity<'_>,
) -> Result<(), AgentError> {
    transaction
        .execute(
            concat!(
                "INSERT INTO planned_test_preparation_identities ",
                "(project_code, schedule_item_code, current_revision_id, created_by, created_at, updated_at) ",
                "VALUES (?1, ?2, NULL, ?3, ?4, ?4) ",
                "ON CONFLICT(schedule_item_code) DO NOTHING"
            ),
            params![
                input.project_code,
                input.schedule_item_code,
                input.created_by,
                input.timestamp
            ],
        )
        .map_err(|error| AgentError::new("planned_test_preparation_write_failed", error.to_string()))?;
    Ok(())
}

pub(crate) fn insert_planned_test_preparation_revision(
    transaction: &Transaction<'_>,
    input: NewPlannedTestPreparationRevision<'_>,
) -> Result<(), AgentError> {
    transaction
        .execute(
            concat!(
                "INSERT INTO planned_test_preparation_revisions ",
                "(revision_id, project_code, schedule_item_code, revision_number, parent_revision_id, ",
                "schedule_revision, method_template_id, method_revision_id, method_definition_checksum, ",
                "station_setup_id, station_setup_revision_id, station_setup_definition_checksum, ",
                "verdict_state, definition_schema_version, definition_json, definition_checksum, ",
                "operation_id, request_checksum, actor, reason, device_id, correlation_id, created_at) ",
                "VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ",
                "?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23)"
            ),
            params![
                input.revision_id,
                input.project_code,
                input.schedule_item_code,
                input.revision_number,
                input.parent_revision_id,
                input.schedule_revision,
                input.method_template_id,
                input.method_revision_id,
                input.method_definition_checksum,
                input.station_setup_id,
                input.station_setup_revision_id,
                input.station_setup_definition_checksum,
                input.verdict_state,
                input.definition_schema_version,
                input.definition_json,
                input.definition_checksum,
                input.operation_id,
                input.request_checksum,
                input.actor,
                input.reason,
                input.device_id,
                input.correlation_id,
                input.timestamp,
            ],
        )
        .map_err(|error| AgentError::new("planned_test_preparation_write_failed", error.to_string()))?;
    Ok(())
}

pub(crate) fn update_current_planned_test_preparation_revision(
    transaction: &Transaction<'_>,
    project_code: &str,
    schedule_item_code: &str,
    expected_current_revision_id: Option<&str>,
    new_revision_id: &str,
    timestamp: &str,
) -> Result<(), AgentError> {
    let updated = if let Some(expected_revision_id) = expected_current_revision_id {
        transaction.execute(
            concat!(
                "UPDATE planned_test_preparation_identities ",
                "SET current_revision_id = ?1, updated_at = ?2 ",
                "WHERE project_code = ?3 AND schedule_item_code = ?4 ",
                "AND current_revision_id = ?5"
            ),
            params![
                new_revision_id,
                timestamp,
                project_code,
                schedule_item_code,
                expected_revision_id
            ],
        )
    } else {
        transaction.execute(
            concat!(
                "UPDATE planned_test_preparation_identities ",
                "SET current_revision_id = ?1, updated_at = ?2 ",
                "WHERE project_code = ?3 AND schedule_item_code = ?4 ",
                "AND current_revision_id IS NULL"
            ),
            params![new_revision_id, timestamp, project_code, schedule_item_code],
        )
    }
    .map_err(|error| AgentError::new("planned_test_preparation_write_failed", error.to_string()))?;
    if updated != 1 {
        return Err(AgentError::new(
            "planned_test_preparation_concurrent_update",
            "the current preparation changed before this assessment was recorded",
        ));
    }
    Ok(())
}

fn revision_select(alias: &str) -> String {
    format!(
        concat!(
            "SELECT {0}.revision_id, {0}.project_code, {0}.schedule_item_code, ",
            "{0}.revision_number, {0}.parent_revision_id, {0}.schedule_revision, ",
            "{0}.method_template_id, {0}.method_revision_id, {0}.method_definition_checksum, ",
            "{0}.station_setup_id, {0}.station_setup_revision_id, ",
            "{0}.station_setup_definition_checksum, {0}.verdict_state, ",
            "{0}.definition_schema_version, {0}.definition_json, {0}.definition_checksum, ",
            "{0}.operation_id, {0}.request_checksum, {0}.actor, {0}.reason, ",
            "{0}.device_id, {0}.correlation_id, {0}.created_at ",
            "FROM planned_test_preparation_revisions {0}"
        ),
        alias
    )
}

fn stored_identity_from_row(
    row: &Row<'_>,
) -> rusqlite::Result<StoredPlannedTestPreparationIdentity> {
    Ok(StoredPlannedTestPreparationIdentity {
        project_code: row.get(0)?,
        schedule_item_code: row.get(1)?,
        current_revision_id: row.get(2)?,
        created_by: row.get(3)?,
        created_at: row.get(4)?,
        updated_at: row.get(5)?,
    })
}

fn stored_revision_from_row(
    row: &Row<'_>,
) -> rusqlite::Result<StoredPlannedTestPreparationRevision> {
    Ok(StoredPlannedTestPreparationRevision {
        revision_id: row.get(0)?,
        project_code: row.get(1)?,
        schedule_item_code: row.get(2)?,
        revision_number: row.get(3)?,
        parent_revision_id: row.get(4)?,
        schedule_revision: row.get(5)?,
        method_template_id: row.get(6)?,
        method_revision_id: row.get(7)?,
        method_definition_checksum: row.get(8)?,
        station_setup_id: row.get(9)?,
        station_setup_revision_id: row.get(10)?,
        station_setup_definition_checksum: row.get(11)?,
        verdict_state: row.get(12)?,
        definition_schema_version: row.get(13)?,
        definition_json: row.get(14)?,
        definition_checksum: row.get(15)?,
        operation_id: row.get(16)?,
        request_checksum: row.get(17)?,
        actor: row.get(18)?,
        reason: row.get(19)?,
        device_id: row.get(20)?,
        correlation_id: row.get(21)?,
        created_at: row.get(22)?,
    })
}
