use crate::{project_repository::table_exists, AgentError};
use emc_locus_core::{
    PlanningValidationIssue, ProjectCode, ScheduleResourceConflictKind, ServiceScheduleItem,
    ServiceScheduleItemInput, ServiceScheduleStatus,
};
use rusqlite::{params, Connection, OptionalExtension, Transaction};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct StoredServiceScheduleItem {
    pub(crate) id: i64,
    pub(crate) item_code: String,
    pub(crate) project_code: String,
    pub(crate) title: String,
    pub(crate) test_category_code: Option<String>,
    pub(crate) test_method_code: Option<String>,
    pub(crate) planned_start_at: String,
    pub(crate) planned_end_at: String,
    pub(crate) assigned_operator: String,
    pub(crate) location: String,
    pub(crate) equipment_under_test: String,
    pub(crate) status: String,
    pub(crate) notes: String,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
    pub(crate) revision: u64,
    pub(crate) created_by: String,
    pub(crate) updated_by: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ScheduleConflict {
    pub(crate) kind: ScheduleResourceConflictKind,
    pub(crate) conflicting_item: StoredServiceScheduleItem,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct StoredLaboratoryScheduleItem {
    pub(crate) schedule_item: StoredServiceScheduleItem,
    pub(crate) customer_name: String,
    pub(crate) project_stage: String,
}

impl StoredServiceScheduleItem {
    pub(crate) fn to_domain(&self) -> Result<ServiceScheduleItem, AgentError> {
        let project_code = ProjectCode::parse(self.project_code.clone()).map_err(|error| {
            AgentError::new(
                "service_schedule_storage_invalid",
                format!("invalid stored project code: {error:?}"),
            )
        })?;
        let status = ServiceScheduleStatus::parse(&self.status).map_err(planning_storage_error)?;
        ServiceScheduleItem::restore(ServiceScheduleItemInput {
            item_code: self.item_code.clone(),
            project_code,
            title: self.title.clone(),
            planned_start_at: self.planned_start_at.clone(),
            planned_end_at: self.planned_end_at.clone(),
            assigned_operator: self.assigned_operator.clone(),
            location: self.location.clone(),
            equipment_under_test: self.equipment_under_test.clone(),
            test_category_code: self.test_category_code.clone(),
            test_method_code: self.test_method_code.clone(),
            status,
            notes: Some(self.notes.clone()),
        })
        .map_err(planning_storage_error)
    }
}

pub(crate) fn ensure_service_schedule_table(connection: &Connection) -> Result<(), AgentError> {
    if !table_exists(connection, "main", "service_schedule_items")? {
        return Err(AgentError::new(
            "storage_not_initialized",
            "missing required table main.service_schedule_items",
        ));
    }
    for column in ["revision", "created_by", "updated_by"] {
        if !column_exists(connection, "service_schedule_items", column)? {
            return Err(AgentError::new(
                "storage_migration_required",
                format!("service schedule migration is missing column {column}"),
            ));
        }
    }
    Ok(())
}

pub(crate) fn load_service_schedule_item(
    connection: &Connection,
    item_code: &str,
) -> Result<Option<StoredServiceScheduleItem>, AgentError> {
    connection
        .query_row(
            &format!("{} WHERE item_code = ?1", schedule_select()),
            params![item_code],
            stored_schedule_item_from_row,
        )
        .optional()
        .map_err(|error| AgentError::new("service_schedule_query_failed", error.to_string()))
}

pub(crate) fn load_project_service_schedule_items(
    connection: &Connection,
    project_code: &str,
) -> Result<Vec<StoredServiceScheduleItem>, AgentError> {
    let mut statement = connection
        .prepare(&format!(
            "{} WHERE project_code = ?1 ORDER BY planned_start_at, planned_end_at, item_code",
            schedule_select()
        ))
        .map_err(|error| AgentError::new("service_schedule_query_failed", error.to_string()))?;
    let rows = statement
        .query_map(params![project_code], stored_schedule_item_from_row)
        .map_err(|error| AgentError::new("service_schedule_query_failed", error.to_string()))?;
    collect_and_validate(rows)
}

pub(crate) fn load_laboratory_service_schedule_items(
    connection: &Connection,
    start_at: &str,
    end_at_exclusive: &str,
) -> Result<Vec<StoredLaboratoryScheduleItem>, AgentError> {
    let mut statement = connection
        .prepare(concat!(
            "SELECT s.id, s.item_code, s.project_code, s.title, s.test_category_code, ",
            "s.test_method_code, s.planned_start_at, s.planned_end_at, ",
            "s.assigned_operator, s.location, s.equipment_under_test, s.status, s.notes, ",
            "s.created_at, s.updated_at, s.revision, s.created_by, s.updated_by, ",
            "p.customer_name, p.stage ",
            "FROM service_schedule_items s JOIN projects p ON p.code = s.project_code ",
            "WHERE s.planned_start_at >= ?1 AND s.planned_start_at < ?2 ",
            "ORDER BY s.planned_start_at, s.planned_end_at, s.project_code, s.item_code"
        ))
        .map_err(|error| AgentError::new("service_schedule_query_failed", error.to_string()))?;
    let rows = statement
        .query_map(params![start_at, end_at_exclusive], |row| {
            Ok(StoredLaboratoryScheduleItem {
                schedule_item: stored_schedule_item_from_row(row)?,
                customer_name: row.get(18)?,
                project_stage: row.get(19)?,
            })
        })
        .map_err(|error| AgentError::new("service_schedule_query_failed", error.to_string()))?;
    let mut items = Vec::new();
    for row in rows {
        let item = row
            .map_err(|error| AgentError::new("service_schedule_query_failed", error.to_string()))?;
        item.schedule_item.to_domain()?;
        items.push(item);
    }
    Ok(items)
}

pub(crate) fn find_service_schedule_conflict(
    connection: &Connection,
    candidate: &ServiceScheduleItem,
    excluded_item_id: Option<i64>,
) -> Result<Option<ScheduleConflict>, AgentError> {
    let mut statement = connection
        .prepare(&format!(
            concat!(
                "{} WHERE planned_start_at < ?1 AND planned_end_at > ?2 ",
                "AND (assigned_operator = ?3 OR location = ?4) ",
                "ORDER BY planned_start_at, planned_end_at, item_code"
            ),
            schedule_select()
        ))
        .map_err(|error| AgentError::new("service_schedule_query_failed", error.to_string()))?;
    let rows = statement
        .query_map(
            params![
                candidate.planned_end_at(),
                candidate.planned_start_at(),
                candidate.assigned_operator(),
                candidate.location()
            ],
            stored_schedule_item_from_row,
        )
        .map_err(|error| AgentError::new("service_schedule_query_failed", error.to_string()))?;
    for row in rows {
        let stored = row
            .map_err(|error| AgentError::new("service_schedule_query_failed", error.to_string()))?;
        if excluded_item_id == Some(stored.id) {
            continue;
        }
        let existing = stored.to_domain()?;
        if let Some(kind) = candidate.resource_conflict(&existing) {
            return Ok(Some(ScheduleConflict {
                kind,
                conflicting_item: stored,
            }));
        }
    }
    Ok(None)
}

pub(crate) fn insert_service_schedule_item(
    transaction: &Transaction<'_>,
    item: &ServiceScheduleItem,
    actor: &str,
    timestamp: &str,
) -> Result<(), AgentError> {
    transaction
        .execute(
            concat!(
                "INSERT INTO service_schedule_items ",
                "(item_code, project_code, title, test_category_code, test_method_code, ",
                "planned_start_at, planned_end_at, assigned_operator, location, ",
                "equipment_under_test, status, notes, created_at, updated_at, revision, ",
                "created_by, updated_by) ",
                "VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ",
                "?13, ?13, 1, ?14, ?14)"
            ),
            params![
                item.item_code(),
                item.project_code().as_str(),
                item.title(),
                item.test_category_code(),
                item.test_method_code(),
                item.planned_start_at(),
                item.planned_end_at(),
                item.assigned_operator(),
                item.location(),
                item.equipment_under_test(),
                item.status().as_str(),
                item.notes(),
                timestamp,
                actor,
            ],
        )
        .map_err(|error| AgentError::new("service_schedule_write_failed", error.to_string()))?;
    Ok(())
}

pub(crate) fn update_service_schedule_status(
    transaction: &Transaction<'_>,
    item_id: i64,
    expected_revision: u64,
    status: ServiceScheduleStatus,
    actor: &str,
    timestamp: &str,
) -> Result<(), AgentError> {
    let updated = transaction
        .execute(
            concat!(
                "UPDATE service_schedule_items SET status = ?1, revision = revision + 1, ",
                "updated_by = ?2, updated_at = ?3 WHERE id = ?4 AND revision = ?5"
            ),
            params![
                status.as_str(),
                actor,
                timestamp,
                item_id,
                expected_revision
            ],
        )
        .map_err(|error| AgentError::new("service_schedule_write_failed", error.to_string()))?;
    if updated != 1 {
        return Err(AgentError::new(
            "service_schedule_concurrent_update",
            "the service schedule item changed before this operation was applied",
        ));
    }
    Ok(())
}

pub(crate) struct StartServiceScheduleInput<'a> {
    pub(crate) item_id: i64,
    pub(crate) project_code: &'a str,
    pub(crate) schedule_item_code: &'a str,
    pub(crate) expected_schedule_revision: u64,
    pub(crate) expected_preparation_revision_id: &'a str,
    pub(crate) expected_preparation_checksum: &'a str,
    pub(crate) actor: &'a str,
    pub(crate) timestamp: &'a str,
}

pub(crate) fn start_service_schedule_with_preparation(
    transaction: &Transaction<'_>,
    input: StartServiceScheduleInput<'_>,
) -> Result<(), AgentError> {
    let updated = transaction
        .execute(
            concat!(
                "UPDATE service_schedule_items ",
                "SET status = 'in_progress', revision = revision + 1, ",
                "updated_by = ?1, updated_at = ?2 ",
                "WHERE id = ?3 AND project_code = ?4 AND item_code = ?5 ",
                "AND revision = ?6 AND status = 'confirmed' ",
                "AND EXISTS (",
                "SELECT 1 FROM planned_test_preparation_identities i ",
                "JOIN planned_test_preparation_revisions r ON r.revision_id = i.current_revision_id ",
                "WHERE i.project_code = ?4 AND i.schedule_item_code = ?5 ",
                "AND i.current_revision_id = ?7 AND r.definition_checksum = ?8 ",
                "AND r.verdict_state = 'ready'",
                ")"
            ),
            params![
                input.actor,
                input.timestamp,
                input.item_id,
                input.project_code,
                input.schedule_item_code,
                input.expected_schedule_revision,
                input.expected_preparation_revision_id,
                input.expected_preparation_checksum,
            ],
        )
        .map_err(|error| AgentError::new("service_schedule_write_failed", error.to_string()))?;
    if updated == 1 {
        return Ok(());
    }

    let current = load_service_schedule_item(transaction, input.schedule_item_code)?;
    if current.as_ref().is_some_and(|item| {
        item.id == input.item_id
            && item.project_code == input.project_code
            && item.revision == input.expected_schedule_revision
            && item.status == "confirmed"
    }) {
        return Err(AgentError::with_details(
            "planned_test_preparation_changed_before_start",
            "La préparation de l'essai a changé pendant le démarrage. Vérifiez-la de nouveau.",
            serde_json::json!({
                "schedule_item_code": input.schedule_item_code,
                "expected_preparation_revision_id": input.expected_preparation_revision_id,
                "expected_preparation_checksum": input.expected_preparation_checksum,
            }),
        ));
    }
    Err(AgentError::new(
        "service_schedule_concurrent_update",
        "the service schedule item changed before this operation was applied",
    ))
}

pub(crate) fn update_service_schedule_assignment(
    transaction: &Transaction<'_>,
    item_id: i64,
    expected_revision: u64,
    item: &ServiceScheduleItem,
    actor: &str,
    timestamp: &str,
) -> Result<(), AgentError> {
    let updated = transaction
        .execute(
            concat!(
                "UPDATE service_schedule_items SET planned_start_at = ?1, ",
                "planned_end_at = ?2, assigned_operator = ?3, location = ?4, ",
                "revision = revision + 1, updated_by = ?5, updated_at = ?6 ",
                "WHERE id = ?7 AND revision = ?8"
            ),
            params![
                item.planned_start_at(),
                item.planned_end_at(),
                item.assigned_operator(),
                item.location(),
                actor,
                timestamp,
                item_id,
                expected_revision,
            ],
        )
        .map_err(|error| AgentError::new("service_schedule_write_failed", error.to_string()))?;
    if updated != 1 {
        return Err(AgentError::new(
            "service_schedule_concurrent_update",
            "the service schedule item changed before this operation was applied",
        ));
    }
    Ok(())
}

fn schedule_select() -> &'static str {
    concat!(
        "SELECT id, item_code, project_code, title, test_category_code, test_method_code, ",
        "planned_start_at, planned_end_at, assigned_operator, location, equipment_under_test, ",
        "status, notes, created_at, updated_at, revision, created_by, updated_by ",
        "FROM service_schedule_items"
    )
}

fn stored_schedule_item_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<StoredServiceScheduleItem> {
    Ok(StoredServiceScheduleItem {
        id: row.get(0)?,
        item_code: row.get(1)?,
        project_code: row.get(2)?,
        title: row.get(3)?,
        test_category_code: row.get(4)?,
        test_method_code: row.get(5)?,
        planned_start_at: row.get(6)?,
        planned_end_at: row.get(7)?,
        assigned_operator: row.get(8)?,
        location: row.get(9)?,
        equipment_under_test: row.get(10)?,
        status: row.get(11)?,
        notes: row.get(12)?,
        created_at: row.get(13)?,
        updated_at: row.get(14)?,
        revision: row.get(15)?,
        created_by: row.get(16)?,
        updated_by: row.get(17)?,
    })
}

fn collect_and_validate(
    rows: rusqlite::MappedRows<
        '_,
        impl FnMut(&rusqlite::Row<'_>) -> rusqlite::Result<StoredServiceScheduleItem>,
    >,
) -> Result<Vec<StoredServiceScheduleItem>, AgentError> {
    let mut items = Vec::new();
    for row in rows {
        let item = row
            .map_err(|error| AgentError::new("service_schedule_query_failed", error.to_string()))?;
        item.to_domain()?;
        items.push(item);
    }
    Ok(items)
}

fn column_exists(connection: &Connection, table: &str, column: &str) -> Result<bool, AgentError> {
    let mut statement = connection
        .prepare(&format!("PRAGMA table_info({table})"))
        .map_err(|error| AgentError::new("database_invalid", error.to_string()))?;
    let names = statement
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|error| AgentError::new("database_invalid", error.to_string()))?;
    for name in names {
        if name.map_err(|error| AgentError::new("database_invalid", error.to_string()))? == column {
            return Ok(true);
        }
    }
    Ok(false)
}

fn planning_storage_error(issue: PlanningValidationIssue) -> AgentError {
    AgentError::new(
        "service_schedule_storage_invalid",
        format!("{}: {}", issue.field, issue.message),
    )
}
