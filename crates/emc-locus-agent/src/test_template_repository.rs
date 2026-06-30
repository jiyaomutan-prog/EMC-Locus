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
pub(crate) struct StoredTestTemplate {
    pub(crate) template_id: String,
    pub(crate) template_revision: String,
    pub(crate) title: String,
    pub(crate) description: String,
    pub(crate) category_code: String,
    pub(crate) measurement_axis: String,
    pub(crate) method_code: Option<String>,
    pub(crate) method_revision: Option<String>,
    pub(crate) status: String,
    pub(crate) variables_json: String,
    pub(crate) lock_policy_json: String,
    pub(crate) instrumentation_chain_json: String,
    pub(crate) sequence_json: String,
    pub(crate) limits_json: String,
    pub(crate) post_processing_json: String,
    pub(crate) created_by: String,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct StoredTestTemplateAuditEvent {
    pub(crate) sequence: u64,
    pub(crate) actor: String,
    pub(crate) action: String,
    pub(crate) reason: String,
    pub(crate) operation_id: String,
    pub(crate) correlation_id: String,
    pub(crate) device_id: String,
    pub(crate) base_revision: String,
    pub(crate) resulting_revision: String,
    pub(crate) payload_json: String,
    pub(crate) occurred_at: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct StoredTestTemplateOperation {
    pub(crate) operation_id: String,
    pub(crate) entity_id: String,
    pub(crate) operation_kind: String,
    pub(crate) base_revision: String,
    pub(crate) actor_id: String,
    pub(crate) device_id: String,
    pub(crate) correlation_id: String,
    pub(crate) payload_checksum: String,
}

pub(crate) struct TestTemplateOperationFingerprintInput<'a> {
    pub(crate) entity_type: &'a str,
    pub(crate) entity_id: &'a str,
    pub(crate) operation_kind: &'a str,
    pub(crate) base_revision: &'a str,
    pub(crate) actor_id: &'a str,
    pub(crate) device_id: &'a str,
    pub(crate) correlation_id: &'a str,
    pub(crate) payload_json: &'a str,
}

pub(crate) struct NewTestTemplateRecord<'a> {
    pub(crate) template_id: &'a str,
    pub(crate) template_revision: &'a str,
    pub(crate) title: &'a str,
    pub(crate) description: &'a str,
    pub(crate) category_code: &'a str,
    pub(crate) measurement_axis: &'a str,
    pub(crate) method_code: Option<&'a str>,
    pub(crate) method_revision: Option<&'a str>,
    pub(crate) status: &'a str,
    pub(crate) variables_json: &'a str,
    pub(crate) lock_policy_json: &'a str,
    pub(crate) instrumentation_chain_json: &'a str,
    pub(crate) sequence_json: &'a str,
    pub(crate) limits_json: &'a str,
    pub(crate) post_processing_json: &'a str,
    pub(crate) created_by: &'a str,
    pub(crate) timestamp: &'a str,
}

pub(crate) struct TestTemplateAuditEventInput<'a> {
    pub(crate) template_id: &'a str,
    pub(crate) sequence: u64,
    pub(crate) actor: &'a str,
    pub(crate) action: &'a str,
    pub(crate) reason: &'a str,
    pub(crate) operation_id: &'a str,
    pub(crate) correlation_id: &'a str,
    pub(crate) device_id: &'a str,
    pub(crate) base_revision: &'a str,
    pub(crate) resulting_revision: &'a str,
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
    pub(crate) status: Option<&'a str>,
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
        "test_templates",
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

pub(crate) fn method_revision_exists(
    connection: &Connection,
    method_code: &str,
    revision: &str,
) -> Result<bool, AgentError> {
    connection
        .query_row(
            concat!(
                "SELECT COUNT(*) FROM test_method_revisions ",
                "WHERE method_code = ?1 AND revision = ?2"
            ),
            params![method_code, revision],
            |row| row.get::<_, u32>(0),
        )
        .map(|count| count > 0)
        .map_err(|error| AgentError::new("test_template_query_failed", error.to_string()))
}

pub(crate) fn load_test_template(
    connection: &Connection,
    template_id: &str,
) -> Result<Option<StoredTestTemplate>, AgentError> {
    connection
        .query_row(
            test_template_select_sql("WHERE template_id = ?1").as_str(),
            params![template_id],
            stored_test_template_from_row,
        )
        .optional()
        .map_err(|error| AgentError::new("test_template_query_failed", error.to_string()))
}

pub(crate) fn list_test_templates(
    connection: &Connection,
    filter: TestTemplateListFilter<'_>,
) -> Result<Vec<StoredTestTemplate>, AgentError> {
    let mut sql = test_template_select_sql("");
    let mut conditions = Vec::new();
    let mut values = Vec::new();
    if let Some(category_code) = filter.category_code {
        conditions.push("category_code = ?");
        values.push(category_code);
    }
    if let Some(status) = filter.status {
        conditions.push("status = ?");
        values.push(status);
    }
    if !conditions.is_empty() {
        sql.push_str(" WHERE ");
        sql.push_str(&conditions.join(" AND "));
    }
    sql.push_str(" ORDER BY category_code, title, template_revision");

    let mut statement = connection
        .prepare(&sql)
        .map_err(|error| AgentError::new("test_template_query_failed", error.to_string()))?;
    let rows = statement
        .query_map(
            rusqlite::params_from_iter(values),
            stored_test_template_from_row,
        )
        .map_err(|error| AgentError::new("test_template_query_failed", error.to_string()))?;
    let mut templates = Vec::new();
    for row in rows {
        templates.push(
            row.map_err(|error| AgentError::new("test_template_query_failed", error.to_string()))?,
        );
    }
    Ok(templates)
}

fn test_template_select_sql(suffix: &str) -> String {
    let base = concat!(
        "SELECT template_id, template_revision, title, description, category_code, ",
        "measurement_axis, method_code, method_revision, status, variables_json, ",
        "lock_policy_json, instrumentation_chain_json, sequence_json, limits_json, ",
        "post_processing_json, created_by, created_at, updated_at FROM test_templates "
    );
    format!("{base}{suffix}")
}

fn stored_test_template_from_row(row: &Row<'_>) -> rusqlite::Result<StoredTestTemplate> {
    Ok(StoredTestTemplate {
        template_id: row.get(0)?,
        template_revision: row.get(1)?,
        title: row.get(2)?,
        description: row.get(3)?,
        category_code: row.get(4)?,
        measurement_axis: row.get(5)?,
        method_code: row.get(6)?,
        method_revision: row.get(7)?,
        status: row.get(8)?,
        variables_json: row.get(9)?,
        lock_policy_json: row.get(10)?,
        instrumentation_chain_json: row.get(11)?,
        sequence_json: row.get(12)?,
        limits_json: row.get(13)?,
        post_processing_json: row.get(14)?,
        created_by: row.get(15)?,
        created_at: row.get(16)?,
        updated_at: row.get(17)?,
    })
}

pub(crate) fn insert_test_template(
    transaction: &Transaction<'_>,
    input: NewTestTemplateRecord<'_>,
) -> Result<(), AgentError> {
    transaction
        .execute(
            concat!(
                "INSERT INTO test_templates ",
                "(template_id, template_revision, title, description, category_code, ",
                "measurement_axis, method_code, method_revision, status, variables_json, ",
                "lock_policy_json, instrumentation_chain_json, sequence_json, limits_json, ",
                "post_processing_json, created_by, created_at, updated_at) ",
                "VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?17)"
            ),
            params![
                input.template_id,
                input.template_revision,
                input.title,
                input.description,
                input.category_code,
                input.measurement_axis,
                input.method_code,
                input.method_revision,
                input.status,
                input.variables_json,
                input.lock_policy_json,
                input.instrumentation_chain_json,
                input.sequence_json,
                input.limits_json,
                input.post_processing_json,
                input.created_by,
                input.timestamp,
            ],
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
                "SELECT operation_id, template_id, action, base_revision, actor, ",
                "device_id, correlation_id, payload_checksum ",
                "FROM test_template_audit_events WHERE operation_id = ?1"
            ),
            params![operation_id],
            |row| {
                Ok(StoredTestTemplateOperation {
                    operation_id: row.get(0)?,
                    entity_id: row.get(1)?,
                    operation_kind: row.get(2)?,
                    base_revision: row.get(3)?,
                    actor_id: row.get(4)?,
                    device_id: row.get(5)?,
                    correlation_id: row.get(6)?,
                    payload_checksum: row.get(7)?,
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
    if operation.entity_id == expected.entity_id
        && operation.operation_kind == expected.operation_kind
        && operation.base_revision == expected.base_revision
        && operation.actor_id == expected.actor_id
        && operation.device_id == expected.device_id
        && operation.correlation_id == expected.correlation_id
        && operation.payload_checksum == expected_fingerprint
    {
        return Ok(());
    }
    Err(AgentError::with_details(
        "operation_replay_mismatch",
        "operation_id is already used for a different canonical test-template operation fingerprint",
        json!({
            "operation_id": operation_id,
            "existing_entity_id": operation.entity_id,
            "existing_operation_kind": operation.operation_kind,
            "existing_base_revision": operation.base_revision,
            "expected_fingerprint": expected_fingerprint,
            "stored_fingerprint": operation.payload_checksum,
        }),
    ))
}

pub(crate) fn next_test_template_audit_sequence(
    connection: &Connection,
    template_id: &str,
) -> Result<u64, AgentError> {
    connection
        .query_row(
            concat!(
                "SELECT COALESCE(MAX(sequence), 0) + 1 ",
                "FROM test_template_audit_events WHERE template_id = ?1"
            ),
            params![template_id],
            |row| row.get(0),
        )
        .map_err(|error| AgentError::new("test_template_audit_query_failed", error.to_string()))
}

pub(crate) fn insert_test_template_audit_event(
    transaction: &Transaction<'_>,
    input: TestTemplateAuditEventInput<'_>,
) -> Result<(), AgentError> {
    let checksum = test_template_operation_fingerprint(&TestTemplateOperationFingerprintInput {
        entity_type: "test_template",
        entity_id: input.template_id,
        operation_kind: input.action,
        base_revision: input.base_revision,
        actor_id: input.actor,
        device_id: input.device_id,
        correlation_id: input.correlation_id,
        payload_json: input.payload_json,
    });
    transaction
        .execute(
            concat!(
                "INSERT INTO test_template_audit_events ",
                "(template_id, sequence, actor, action, reason, operation_id, correlation_id, ",
                "device_id, base_revision, resulting_revision, payload_json, payload_checksum, occurred_at) ",
                "VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)"
            ),
            params![
                input.template_id,
                input.sequence,
                input.actor,
                input.action,
                input.reason,
                input.operation_id,
                input.correlation_id,
                input.device_id,
                input.base_revision,
                input.resulting_revision,
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
    let checksum = test_template_operation_fingerprint(&TestTemplateOperationFingerprintInput {
        entity_type: input.entity_type,
        entity_id: input.entity_id,
        operation_kind: input.operation_kind,
        base_revision: input.base_revision,
        actor_id: input.actor_id,
        device_id: input.device_id,
        correlation_id: input.correlation_id,
        payload_json: input.payload_json,
    });
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
            "SELECT sequence, actor, action, reason, operation_id, correlation_id, ",
            "device_id, base_revision, resulting_revision, payload_json, occurred_at ",
            "FROM test_template_audit_events WHERE template_id = ?1 ORDER BY sequence"
        ))
        .map_err(|error| AgentError::new("test_template_audit_query_failed", error.to_string()))?;
    let rows = statement
        .query_map(params![template_id], |row| {
            Ok(StoredTestTemplateAuditEvent {
                sequence: row.get(0)?,
                actor: row.get(1)?,
                action: row.get(2)?,
                reason: row.get(3)?,
                operation_id: row.get(4)?,
                correlation_id: row.get(5)?,
                device_id: row.get(6)?,
                base_revision: row.get(7)?,
                resulting_revision: row.get(8)?,
                payload_json: row.get(9)?,
                occurred_at: row.get(10)?,
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
        "entity_id": input.entity_id,
        "operation_kind": input.operation_kind,
        "base_revision": input.base_revision,
        "actor_id": input.actor_id,
        "device_id": input.device_id,
        "correlation_id": input.correlation_id,
        "payload": serde_json::from_str::<serde_json::Value>(input.payload_json)
            .expect("canonical test-template operation payload must be valid JSON"),
    })))
}

fn payload_checksum(payload_json: &str) -> String {
    let digest = Sha256::digest(payload_json.as_bytes());
    format!("sha256:{digest:x}")
}
