use crate::{
    render_json,
    sqlite_policy::{enforce_project_slice_journal_mode, AttachedDatabase},
    AgentError,
};
use rusqlite::{params, Connection, OptionalExtension, Row, Transaction};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::path::Path;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct StoredTestTemplateIdentity {
    pub(crate) template_id: String,
    pub(crate) title: String,
    pub(crate) category_code: String,
    pub(crate) current_approved_revision_id: Option<String>,
    pub(crate) created_by: String,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct StoredTestTemplateRevision {
    pub(crate) revision_id: String,
    pub(crate) template_id: String,
    pub(crate) revision_number: u32,
    pub(crate) parent_revision_id: Option<String>,
    pub(crate) status: String,
    pub(crate) definition_schema_version: String,
    pub(crate) definition_json: String,
    pub(crate) definition_checksum: String,
    pub(crate) created_by: String,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
    pub(crate) submitted_at: Option<String>,
    pub(crate) approved_at: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct StoredTestTemplateAggregate {
    pub(crate) identity: StoredTestTemplateIdentity,
    pub(crate) current_revision: Option<StoredTestTemplateRevision>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct StoredTestTemplateAuditEvent {
    pub(crate) audit_id: u64,
    pub(crate) template_id: String,
    pub(crate) revision_id: Option<String>,
    pub(crate) action: String,
    pub(crate) actor: String,
    pub(crate) reason: String,
    pub(crate) old_revision_id: Option<String>,
    pub(crate) new_revision_id: Option<String>,
    pub(crate) old_definition_checksum: Option<String>,
    pub(crate) new_definition_checksum: Option<String>,
    pub(crate) operation_id: String,
    pub(crate) device_id: String,
    pub(crate) correlation_id: String,
    pub(crate) payload_json: String,
    pub(crate) occurred_at: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct StoredTestTemplateOperation {
    pub(crate) operation_id: String,
    pub(crate) template_id: String,
    pub(crate) revision_id: Option<String>,
    pub(crate) action: String,
    pub(crate) actor: String,
    pub(crate) device_id: String,
    pub(crate) correlation_id: String,
    pub(crate) old_revision_id: Option<String>,
    pub(crate) new_revision_id: Option<String>,
    pub(crate) old_definition_checksum: Option<String>,
    pub(crate) new_definition_checksum: Option<String>,
    pub(crate) payload_checksum: String,
}

pub(crate) struct TestTemplateOperationFingerprintInput<'a> {
    pub(crate) entity_type: &'a str,
    pub(crate) template_id: &'a str,
    pub(crate) revision_id: Option<&'a str>,
    pub(crate) action: &'a str,
    pub(crate) actor: &'a str,
    pub(crate) device_id: &'a str,
    pub(crate) correlation_id: &'a str,
    pub(crate) old_revision_id: Option<&'a str>,
    pub(crate) new_revision_id: Option<&'a str>,
    pub(crate) old_definition_checksum: Option<&'a str>,
    pub(crate) new_definition_checksum: Option<&'a str>,
    pub(crate) payload_json: &'a str,
}

pub(crate) struct NewTestTemplateIdentityRecord<'a> {
    pub(crate) template_id: &'a str,
    pub(crate) title: &'a str,
    pub(crate) category_code: &'a str,
    pub(crate) created_by: &'a str,
    pub(crate) timestamp: &'a str,
}

pub(crate) struct NewTestTemplateRevisionRecord<'a> {
    pub(crate) revision_id: &'a str,
    pub(crate) template_id: &'a str,
    pub(crate) revision_number: u32,
    pub(crate) parent_revision_id: Option<&'a str>,
    pub(crate) status: &'a str,
    pub(crate) definition_schema_version: &'a str,
    pub(crate) definition_json: &'a str,
    pub(crate) definition_checksum: &'a str,
    pub(crate) created_by: &'a str,
    pub(crate) timestamp: &'a str,
}

pub(crate) struct UpdateTestTemplateRevisionDefinitionInput<'a> {
    pub(crate) revision_id: &'a str,
    pub(crate) definition_schema_version: &'a str,
    pub(crate) definition_json: &'a str,
    pub(crate) definition_checksum: &'a str,
    pub(crate) timestamp: &'a str,
}

pub(crate) struct UpdateTestTemplateRevisionStatusInput<'a> {
    pub(crate) template_id: &'a str,
    pub(crate) revision_id: &'a str,
    pub(crate) status: &'a str,
    pub(crate) timestamp: &'a str,
}

pub(crate) struct TestTemplateAuditEventInput<'a> {
    pub(crate) template_id: &'a str,
    pub(crate) revision_id: Option<&'a str>,
    pub(crate) action: &'a str,
    pub(crate) actor: &'a str,
    pub(crate) reason: &'a str,
    pub(crate) operation_id: &'a str,
    pub(crate) correlation_id: &'a str,
    pub(crate) device_id: &'a str,
    pub(crate) old_revision_id: Option<&'a str>,
    pub(crate) new_revision_id: Option<&'a str>,
    pub(crate) old_definition_checksum: Option<&'a str>,
    pub(crate) new_definition_checksum: Option<&'a str>,
    pub(crate) payload_json: &'a str,
    pub(crate) timestamp: &'a str,
}

pub(crate) struct TestTemplateSyncOperationInput<'a> {
    pub(crate) operation_id: &'a str,
    pub(crate) entity_type: &'a str,
    pub(crate) entity_id: &'a str,
    pub(crate) operation_kind: &'a str,
    pub(crate) base_revision: &'a str,
    pub(crate) resulting_revision: &'a str,
    pub(crate) actor_id: &'a str,
    pub(crate) device_id: &'a str,
    pub(crate) correlation_id: &'a str,
    pub(crate) payload_json: &'a str,
    pub(crate) timestamp: &'a str,
}

#[derive(Default)]
pub(crate) struct TestTemplateListFilter<'a> {
    pub(crate) category_code: Option<&'a str>,
}

pub(crate) fn open_test_template_connection(storage_root: &Path) -> Result<Connection, AgentError> {
    let database = storage_root.join("test_definitions.sqlite");
    if !database.exists() {
        return Err(AgentError::new(
            "storage_not_initialized",
            "test-template reads require initialized test_definitions.sqlite",
        ));
    }
    let connection = Connection::open(&database).map_err(|error| {
        AgentError::new(
            "database_open_error",
            format!("cannot open {}: {error}", database.display()),
        )
    })?;
    connection
        .execute_batch("PRAGMA foreign_keys = ON;")
        .map_err(|error| AgentError::new("database_pragma_error", error.to_string()))?;
    ensure_test_template_tables(&connection)?;
    Ok(connection)
}

pub(crate) fn open_test_template_connection_with_sync(
    storage_root: &Path,
) -> Result<Connection, AgentError> {
    let test_definitions_database = storage_root.join("test_definitions.sqlite");
    let sync_database = storage_root.join("sync.sqlite");
    if !test_definitions_database.exists() || !sync_database.exists() {
        return Err(AgentError::new(
            "storage_not_initialized",
            "test-template writes require initialized test_definitions.sqlite and sync.sqlite",
        ));
    }
    let connection = Connection::open(&test_definitions_database).map_err(|error| {
        AgentError::new(
            "database_open_error",
            format!(
                "cannot open {}: {error}",
                test_definitions_database.display()
            ),
        )
    })?;
    connection
        .execute_batch("PRAGMA foreign_keys = ON;")
        .map_err(|error| AgentError::new("database_pragma_error", error.to_string()))?;
    let sync_path = sync_database.to_string_lossy().to_string();
    connection
        .execute("ATTACH DATABASE ?1 AS sync_db", params![sync_path])
        .map_err(|error| AgentError::new("database_attach_error", error.to_string()))?;
    enforce_project_slice_journal_mode(
        &connection,
        AttachedDatabase::Main,
        "test_definitions.sqlite",
    )?;
    enforce_project_slice_journal_mode(&connection, AttachedDatabase::SyncDb, "sync.sqlite")?;
    ensure_test_template_tables(&connection)?;
    ensure_sync_tables(&connection)?;
    Ok(connection)
}

fn ensure_test_template_tables(connection: &Connection) -> Result<(), AgentError> {
    for table in [
        "schema_migrations",
        "repository_metadata",
        "standards",
        "test_methods",
        "test_method_revisions",
        "test_categories",
        "test_template_identities",
        "test_template_revisions",
        "test_template_audit_events",
    ] {
        if !table_exists_in_schema(connection, "main", table)? {
            return Err(AgentError::new(
                "storage_not_initialized",
                format!("missing required test_definitions table {table}"),
            ));
        }
    }
    Ok(())
}

fn ensure_sync_tables(connection: &Connection) -> Result<(), AgentError> {
    if !table_exists_in_schema(connection, "sync_db", "sync_operations")? {
        return Err(AgentError::new(
            "storage_not_initialized",
            "missing required table sync_db.sync_operations",
        ));
    }
    Ok(())
}

fn table_exists_in_schema(
    connection: &Connection,
    schema: &str,
    table: &str,
) -> Result<bool, AgentError> {
    let sql =
        format!("SELECT COUNT(*) FROM {schema}.sqlite_master WHERE type = 'table' AND name = ?1");
    let count: u32 = connection
        .query_row(&sql, params![table], |row| row.get(0))
        .map_err(|error| AgentError::new("database_invalid", error.to_string()))?;
    Ok(count > 0)
}

pub(crate) fn test_category_exists(
    connection: &Connection,
    category_code: &str,
) -> Result<bool, AgentError> {
    connection
        .query_row(
            "SELECT COUNT(*) FROM test_categories WHERE code = ?1 AND active = 1",
            params![category_code],
            |row| row.get::<_, u32>(0),
        )
        .map(|count| count > 0)
        .map_err(|error| AgentError::new("test_template_query_failed", error.to_string()))
}

pub(crate) fn approved_method_revision_exists(
    connection: &Connection,
    method_code: &str,
    revision: &str,
) -> Result<bool, AgentError> {
    connection
        .query_row(
            concat!(
                "SELECT COUNT(*) FROM test_method_revisions ",
                "WHERE method_code = ?1 AND revision = ?2 AND status = 'approved'"
            ),
            params![method_code, revision],
            |row| row.get::<_, u32>(0),
        )
        .map(|count| count > 0)
        .map_err(|error| AgentError::new("test_template_query_failed", error.to_string()))
}

pub(crate) fn load_test_template_identity(
    connection: &Connection,
    template_id: &str,
) -> Result<Option<StoredTestTemplateIdentity>, AgentError> {
    connection
        .query_row(
            identity_select_sql("WHERE template_id = ?1").as_str(),
            params![template_id],
            stored_identity_from_row,
        )
        .optional()
        .map_err(|error| AgentError::new("test_template_query_failed", error.to_string()))
}

pub(crate) fn list_test_template_identities(
    connection: &Connection,
    filter: TestTemplateListFilter<'_>,
) -> Result<Vec<StoredTestTemplateIdentity>, AgentError> {
    let mut sql = identity_select_sql("");
    let mut values = Vec::new();
    if let Some(category_code) = filter.category_code {
        sql.push_str(" WHERE category_code = ?");
        values.push(category_code);
    }
    sql.push_str(" ORDER BY category_code, title, template_id");
    let mut statement = connection
        .prepare(&sql)
        .map_err(|error| AgentError::new("test_template_query_failed", error.to_string()))?;
    let rows = statement
        .query_map(rusqlite::params_from_iter(values), stored_identity_from_row)
        .map_err(|error| AgentError::new("test_template_query_failed", error.to_string()))?;
    let mut identities = Vec::new();
    for row in rows {
        identities.push(
            row.map_err(|error| AgentError::new("test_template_query_failed", error.to_string()))?,
        );
    }
    Ok(identities)
}

fn identity_select_sql(suffix: &str) -> String {
    let base = concat!(
        "SELECT template_id, title, category_code, current_approved_revision_id, ",
        "created_by, created_at, updated_at FROM test_template_identities "
    );
    format!("{base}{suffix}")
}

fn stored_identity_from_row(row: &Row<'_>) -> rusqlite::Result<StoredTestTemplateIdentity> {
    Ok(StoredTestTemplateIdentity {
        template_id: row.get(0)?,
        title: row.get(1)?,
        category_code: row.get(2)?,
        current_approved_revision_id: row.get(3)?,
        created_by: row.get(4)?,
        created_at: row.get(5)?,
        updated_at: row.get(6)?,
    })
}

pub(crate) fn load_test_template_revision(
    connection: &Connection,
    template_id: &str,
    revision_id: &str,
) -> Result<Option<StoredTestTemplateRevision>, AgentError> {
    connection
        .query_row(
            revision_select_sql("WHERE template_id = ?1 AND revision_id = ?2").as_str(),
            params![template_id, revision_id],
            stored_revision_from_row,
        )
        .optional()
        .map_err(|error| AgentError::new("test_template_revision_query_failed", error.to_string()))
}

pub(crate) fn load_current_test_template_revision(
    connection: &Connection,
    identity: &StoredTestTemplateIdentity,
) -> Result<Option<StoredTestTemplateRevision>, AgentError> {
    if let Some(revision_id) = identity.current_approved_revision_id.as_deref() {
        return load_test_template_revision(connection, &identity.template_id, revision_id);
    }
    connection
        .query_row(
            revision_select_sql("WHERE template_id = ?1 ORDER BY revision_number DESC LIMIT 1")
                .as_str(),
            params![identity.template_id],
            stored_revision_from_row,
        )
        .optional()
        .map_err(|error| AgentError::new("test_template_revision_query_failed", error.to_string()))
}

pub(crate) fn list_test_template_revisions(
    connection: &Connection,
    template_id: &str,
) -> Result<Vec<StoredTestTemplateRevision>, AgentError> {
    let mut statement = connection
        .prepare(revision_select_sql("WHERE template_id = ?1 ORDER BY revision_number").as_str())
        .map_err(|error| {
            AgentError::new("test_template_revision_query_failed", error.to_string())
        })?;
    let rows = statement
        .query_map(params![template_id], stored_revision_from_row)
        .map_err(|error| {
            AgentError::new("test_template_revision_query_failed", error.to_string())
        })?;
    let mut revisions = Vec::new();
    for row in rows {
        revisions.push(row.map_err(|error| {
            AgentError::new("test_template_revision_query_failed", error.to_string())
        })?);
    }
    Ok(revisions)
}

fn revision_select_sql(suffix: &str) -> String {
    let base = concat!(
        "SELECT revision_id, template_id, revision_number, parent_revision_id, status, ",
        "definition_schema_version, definition_json, definition_checksum, created_by, ",
        "created_at, updated_at, submitted_at, approved_at FROM test_template_revisions "
    );
    format!("{base}{suffix}")
}

fn stored_revision_from_row(row: &Row<'_>) -> rusqlite::Result<StoredTestTemplateRevision> {
    Ok(StoredTestTemplateRevision {
        revision_id: row.get(0)?,
        template_id: row.get(1)?,
        revision_number: row.get(2)?,
        parent_revision_id: row.get(3)?,
        status: row.get(4)?,
        definition_schema_version: row.get(5)?,
        definition_json: row.get(6)?,
        definition_checksum: row.get(7)?,
        created_by: row.get(8)?,
        created_at: row.get(9)?,
        updated_at: row.get(10)?,
        submitted_at: row.get(11)?,
        approved_at: row.get(12)?,
    })
}

pub(crate) fn next_test_template_revision_number(
    connection: &Connection,
    template_id: &str,
) -> Result<u32, AgentError> {
    connection
        .query_row(
            "SELECT COALESCE(MAX(revision_number), 0) + 1 FROM test_template_revisions WHERE template_id = ?1",
            params![template_id],
            |row| row.get::<_, u32>(0),
        )
        .map_err(|error| AgentError::new("test_template_revision_query_failed", error.to_string()))
}

pub(crate) fn insert_test_template_identity(
    transaction: &Transaction<'_>,
    input: NewTestTemplateIdentityRecord<'_>,
) -> Result<(), AgentError> {
    transaction
        .execute(
            concat!(
                "INSERT INTO test_template_identities ",
                "(template_id, title, category_code, created_by, created_at, updated_at) ",
                "VALUES (?1, ?2, ?3, ?4, ?5, ?5)"
            ),
            params![
                input.template_id,
                input.title,
                input.category_code,
                input.created_by,
                input.timestamp,
            ],
        )
        .map_err(|error| AgentError::new("test_template_write_failed", error.to_string()))?;
    Ok(())
}

pub(crate) fn insert_test_template_revision(
    transaction: &Transaction<'_>,
    input: NewTestTemplateRevisionRecord<'_>,
) -> Result<(), AgentError> {
    transaction
        .execute(
            concat!(
                "INSERT INTO test_template_revisions ",
                "(revision_id, template_id, revision_number, parent_revision_id, status, ",
                "definition_schema_version, definition_json, definition_checksum, created_by, ",
                "created_at, updated_at) ",
                "VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?10)"
            ),
            params![
                input.revision_id,
                input.template_id,
                input.revision_number,
                input.parent_revision_id,
                input.status,
                input.definition_schema_version,
                input.definition_json,
                input.definition_checksum,
                input.created_by,
                input.timestamp,
            ],
        )
        .map_err(|error| {
            AgentError::new("test_template_revision_write_failed", error.to_string())
        })?;
    Ok(())
}

pub(crate) fn update_test_template_revision_definition(
    transaction: &Transaction<'_>,
    input: UpdateTestTemplateRevisionDefinitionInput<'_>,
) -> Result<(), AgentError> {
    let updated = transaction
        .execute(
            concat!(
                "UPDATE test_template_revisions SET definition_schema_version = ?2, ",
                "definition_json = ?3, definition_checksum = ?4, updated_at = ?5 ",
                "WHERE revision_id = ?1 AND status = 'draft'"
            ),
            params![
                input.revision_id,
                input.definition_schema_version,
                input.definition_json,
                input.definition_checksum,
                input.timestamp,
            ],
        )
        .map_err(|error| {
            AgentError::new("test_template_revision_write_failed", error.to_string())
        })?;
    if updated == 0 {
        return Err(AgentError::new(
            "test_template_revision_immutable",
            "only draft template revisions can be modified",
        ));
    }
    Ok(())
}

pub(crate) fn update_test_template_revision_status(
    transaction: &Transaction<'_>,
    input: UpdateTestTemplateRevisionStatusInput<'_>,
) -> Result<(), AgentError> {
    let (submitted_at, approved_at): (Option<&str>, Option<&str>) = match input.status {
        "under_review" => (Some(input.timestamp), None),
        "approved" => (Some(input.timestamp), Some(input.timestamp)),
        _ => (None, None),
    };
    let updated = transaction
        .execute(
            concat!(
                "UPDATE test_template_revisions SET status = ?3, updated_at = ?4, ",
                "submitted_at = COALESCE(submitted_at, ?5), approved_at = COALESCE(approved_at, ?6) ",
                "WHERE template_id = ?1 AND revision_id = ?2"
            ),
            params![
                input.template_id,
                input.revision_id,
                input.status,
                input.timestamp,
                submitted_at,
                approved_at,
            ],
        )
        .map_err(|error| AgentError::new("test_template_revision_write_failed", error.to_string()))?;
    if updated == 0 {
        return Err(AgentError::new(
            "test_template_revision_not_found",
            "template revision does not exist",
        ));
    }
    if input.status == "approved" {
        transaction
            .execute(
                concat!(
                    "UPDATE test_template_identities SET current_approved_revision_id = ?2, ",
                    "updated_at = ?3 WHERE template_id = ?1"
                ),
                params![input.template_id, input.revision_id, input.timestamp],
            )
            .map_err(|error| AgentError::new("test_template_write_failed", error.to_string()))?;
    } else {
        transaction
            .execute(
                "UPDATE test_template_identities SET updated_at = ?2 WHERE template_id = ?1",
                params![input.template_id, input.timestamp],
            )
            .map_err(|error| AgentError::new("test_template_write_failed", error.to_string()))?;
    }
    Ok(())
}

pub(crate) fn touch_test_template_identity(
    transaction: &Transaction<'_>,
    template_id: &str,
    timestamp: &str,
) -> Result<(), AgentError> {
    transaction
        .execute(
            "UPDATE test_template_identities SET updated_at = ?2 WHERE template_id = ?1",
            params![template_id, timestamp],
        )
        .map_err(|error| AgentError::new("test_template_write_failed", error.to_string()))?;
    Ok(())
}

pub(crate) fn existing_test_template_operation(
    connection: &Connection,
    operation_id: &str,
) -> Result<Option<StoredTestTemplateOperation>, AgentError> {
    connection
        .query_row(
            concat!(
                "SELECT operation_id, template_id, revision_id, action, actor, device_id, ",
                "correlation_id, old_revision_id, new_revision_id, old_definition_checksum, ",
                "new_definition_checksum, payload_checksum ",
                "FROM test_template_audit_events WHERE operation_id = ?1"
            ),
            params![operation_id],
            |row| {
                Ok(StoredTestTemplateOperation {
                    operation_id: row.get(0)?,
                    template_id: row.get(1)?,
                    revision_id: row.get(2)?,
                    action: row.get(3)?,
                    actor: row.get(4)?,
                    device_id: row.get(5)?,
                    correlation_id: row.get(6)?,
                    old_revision_id: row.get(7)?,
                    new_revision_id: row.get(8)?,
                    old_definition_checksum: row.get(9)?,
                    new_definition_checksum: row.get(10)?,
                    payload_checksum: row.get(11)?,
                })
            },
        )
        .optional()
        .map_err(|error| AgentError::new("test_template_audit_query_failed", error.to_string()))
}

pub(crate) fn ensure_test_template_operation_replay(
    operation: &StoredTestTemplateOperation,
    operation_id: &str,
    expected: TestTemplateOperationFingerprintInput<'_>,
) -> Result<(), AgentError> {
    let expected_fingerprint = test_template_operation_fingerprint(&expected);
    if operation.template_id == expected.template_id
        && operation.revision_id.as_deref() == expected.revision_id
        && operation.action == expected.action
        && operation.actor == expected.actor
        && operation.device_id == expected.device_id
        && operation.correlation_id == expected.correlation_id
        && operation.old_revision_id.as_deref() == expected.old_revision_id
        && operation.new_revision_id.as_deref() == expected.new_revision_id
        && operation.old_definition_checksum.as_deref() == expected.old_definition_checksum
        && operation.new_definition_checksum.as_deref() == expected.new_definition_checksum
        && operation.payload_checksum == expected_fingerprint
    {
        return Ok(());
    }
    Err(AgentError::with_details(
        "operation_replay_mismatch",
        "operation_id is already used for a different canonical test-template operation fingerprint",
        json!({
            "operation_id": operation_id,
            "existing_template_id": operation.template_id,
            "existing_revision_id": operation.revision_id,
            "existing_action": operation.action,
            "expected_action": expected.action,
            "expected_fingerprint": expected_fingerprint,
            "stored_fingerprint": operation.payload_checksum,
        }),
    ))
}

pub(crate) fn insert_test_template_audit_event(
    transaction: &Transaction<'_>,
    input: TestTemplateAuditEventInput<'_>,
) -> Result<(), AgentError> {
    let checksum = test_template_operation_fingerprint(&TestTemplateOperationFingerprintInput {
        entity_type: "test_template_revision",
        template_id: input.template_id,
        revision_id: input.revision_id,
        action: input.action,
        actor: input.actor,
        device_id: input.device_id,
        correlation_id: input.correlation_id,
        old_revision_id: input.old_revision_id,
        new_revision_id: input.new_revision_id,
        old_definition_checksum: input.old_definition_checksum,
        new_definition_checksum: input.new_definition_checksum,
        payload_json: input.payload_json,
    });
    transaction
        .execute(
            concat!(
                "INSERT INTO test_template_audit_events ",
                "(template_id, revision_id, action, actor, reason, old_revision_id, ",
                "new_revision_id, old_definition_checksum, new_definition_checksum, ",
                "operation_id, device_id, correlation_id, payload_json, payload_checksum, occurred_at) ",
                "VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)"
            ),
            params![
                input.template_id,
                input.revision_id,
                input.action,
                input.actor,
                input.reason,
                input.old_revision_id,
                input.new_revision_id,
                input.old_definition_checksum,
                input.new_definition_checksum,
                input.operation_id,
                input.device_id,
                input.correlation_id,
                input.payload_json,
                checksum,
                input.timestamp,
            ],
        )
        .map_err(|error| AgentError::new("test_template_audit_write_failed", error.to_string()))?;
    Ok(())
}

pub(crate) fn insert_test_template_sync_operation(
    transaction: &Transaction<'_>,
    input: TestTemplateSyncOperationInput<'_>,
) -> Result<(), AgentError> {
    let checksum = payload_checksum(&render_json(&json!({
        "domain": "test_definitions",
        "entity_type": input.entity_type,
        "entity_id": input.entity_id,
        "operation_kind": input.operation_kind,
        "base_revision": input.base_revision,
        "resulting_revision": input.resulting_revision,
        "actor_id": input.actor_id,
        "device_id": input.device_id,
        "correlation_id": input.correlation_id,
        "payload": serde_json::from_str::<serde_json::Value>(input.payload_json)
            .expect("canonical test-template operation payload must be valid JSON"),
    })));
    transaction
        .execute(
            concat!(
                "INSERT INTO sync_db.sync_operations ",
                "(operation_id, domain, entity_type, entity_id, operation_kind, ",
                "base_revision, resulting_revision, actor_id, device_id, correlation_id, ",
                "payload_json, payload_checksum, status, occurred_at, recorded_at) ",
                "VALUES (?1, 'test_definitions', ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, 'pending', ?12, ?12)"
            ),
            params![
                input.operation_id,
                input.entity_type,
                input.entity_id,
                input.operation_kind,
                input.base_revision,
                input.resulting_revision,
                input.actor_id,
                input.device_id,
                input.correlation_id,
                input.payload_json,
                checksum,
                input.timestamp,
            ],
        )
        .map_err(|error| AgentError::new("sync_outbox_write_failed", error.to_string()))?;
    Ok(())
}

pub(crate) fn load_test_template_audit_events(
    connection: &Connection,
    template_id: &str,
) -> Result<Vec<StoredTestTemplateAuditEvent>, AgentError> {
    let mut statement = connection
        .prepare(concat!(
            "SELECT audit_id, template_id, revision_id, action, actor, reason, ",
            "old_revision_id, new_revision_id, old_definition_checksum, ",
            "new_definition_checksum, operation_id, device_id, correlation_id, ",
            "payload_json, occurred_at FROM test_template_audit_events ",
            "WHERE template_id = ?1 ORDER BY audit_id"
        ))
        .map_err(|error| AgentError::new("test_template_audit_query_failed", error.to_string()))?;
    let rows = statement
        .query_map(params![template_id], |row| {
            Ok(StoredTestTemplateAuditEvent {
                audit_id: row.get(0)?,
                template_id: row.get(1)?,
                revision_id: row.get(2)?,
                action: row.get(3)?,
                actor: row.get(4)?,
                reason: row.get(5)?,
                old_revision_id: row.get(6)?,
                new_revision_id: row.get(7)?,
                old_definition_checksum: row.get(8)?,
                new_definition_checksum: row.get(9)?,
                operation_id: row.get(10)?,
                device_id: row.get(11)?,
                correlation_id: row.get(12)?,
                payload_json: row.get(13)?,
                occurred_at: row.get(14)?,
            })
        })
        .map_err(|error| AgentError::new("test_template_audit_query_failed", error.to_string()))?;
    let mut events = Vec::new();
    for row in rows {
        events.push(row.map_err(|error| {
            AgentError::new("test_template_audit_query_failed", error.to_string())
        })?);
    }
    Ok(events)
}

fn test_template_operation_fingerprint(
    input: &TestTemplateOperationFingerprintInput<'_>,
) -> String {
    payload_checksum(&render_json(&json!({
        "domain": "test_definitions",
        "entity_type": input.entity_type,
        "template_id": input.template_id,
        "revision_id": input.revision_id,
        "action": input.action,
        "actor": input.actor,
        "device_id": input.device_id,
        "correlation_id": input.correlation_id,
        "old_revision_id": input.old_revision_id,
        "new_revision_id": input.new_revision_id,
        "old_definition_checksum": input.old_definition_checksum,
        "new_definition_checksum": input.new_definition_checksum,
        "payload": serde_json::from_str::<serde_json::Value>(input.payload_json)
            .expect("canonical test-template operation payload must be valid JSON"),
    })))
}

fn payload_checksum(payload_json: &str) -> String {
    let digest = Sha256::digest(payload_json.as_bytes());
    format!("sha256:{digest:x}")
}
