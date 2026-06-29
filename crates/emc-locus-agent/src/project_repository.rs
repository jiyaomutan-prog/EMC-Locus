use crate::{render_json, AgentError};
use rusqlite::{params, Connection, OptionalExtension, Transaction};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::path::Path;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct StoredProject {
    pub(crate) code: String,
    pub(crate) customer_name: String,
    pub(crate) stage: String,
    pub(crate) execution_mode: String,
    pub(crate) created_at: String,
    pub(crate) archived_at: Option<String>,
    pub(crate) revision_sequence: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct StoredOperation {
    pub(crate) operation_id: String,
    pub(crate) entity_id: String,
    pub(crate) operation_kind: String,
    pub(crate) base_revision: String,
    pub(crate) resulting_revision: String,
    pub(crate) actor_id: String,
    pub(crate) device_id: String,
    pub(crate) correlation_id: String,
    pub(crate) payload_checksum: String,
}

pub(crate) struct OperationFingerprintInput<'a> {
    pub(crate) entity_id: &'a str,
    pub(crate) operation_kind: &'a str,
    pub(crate) base_revision: &'a str,
    pub(crate) actor_id: &'a str,
    pub(crate) device_id: &'a str,
    pub(crate) correlation_id: &'a str,
    pub(crate) payload_json: &'a str,
}

pub(crate) struct AuditEventInput<'a> {
    pub(crate) project_code: &'a str,
    pub(crate) sequence: u64,
    pub(crate) actor: &'a str,
    pub(crate) action: &'a str,
    pub(crate) reason: Option<&'a str>,
    pub(crate) payload_json: &'a str,
    pub(crate) timestamp: &'a str,
}

pub(crate) struct SyncOperationInput<'a> {
    pub(crate) operation_id: &'a str,
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

pub(crate) fn open_project_connection(storage_root: &Path) -> Result<Connection, AgentError> {
    let projects_database = storage_root.join("projects.sqlite");
    let sync_database = storage_root.join("sync.sqlite");
    if !projects_database.exists() || !sync_database.exists() {
        return Err(AgentError::new(
            "storage_not_initialized",
            "project commands require initialized projects.sqlite and sync.sqlite",
        ));
    }
    let connection = Connection::open(&projects_database).map_err(|error| {
        AgentError::new(
            "database_open_error",
            format!("cannot open {}: {error}", projects_database.display()),
        )
    })?;
    connection
        .execute_batch("PRAGMA foreign_keys = ON;")
        .map_err(|error| AgentError::new("database_pragma_error", error.to_string()))?;
    let sync_path = sync_database.to_string_lossy().to_string();
    connection
        .execute("ATTACH DATABASE ?1 AS sync_db", params![sync_path])
        .map_err(|error| AgentError::new("database_attach_error", error.to_string()))?;
    ensure_project_tables(&connection)?;
    Ok(connection)
}

fn ensure_project_tables(connection: &Connection) -> Result<(), AgentError> {
    for (schema, table) in [
        ("main", "projects"),
        ("main", "project_audit_events"),
        ("main", "contract_review_items"),
        ("sync_db", "sync_operations"),
    ] {
        if !table_exists(connection, schema, table)? {
            return Err(AgentError::new(
                "storage_not_initialized",
                format!("missing required table {schema}.{table}"),
            ));
        }
    }
    Ok(())
}

fn table_exists(connection: &Connection, schema: &str, table: &str) -> Result<bool, AgentError> {
    let sql =
        format!("SELECT COUNT(*) FROM {schema}.sqlite_master WHERE type = 'table' AND name = ?1");
    let count: u32 = connection
        .query_row(&sql, params![table], |row| row.get(0))
        .map_err(|error| AgentError::new("database_invalid", error.to_string()))?;
    Ok(count > 0)
}

pub(crate) fn load_project(
    connection: &Connection,
    code: &str,
) -> Result<Option<StoredProject>, AgentError> {
    connection
        .query_row(
            concat!(
                "SELECT p.code, p.customer_name, p.stage, p.execution_mode, p.created_at, p.archived_at, ",
                "COALESCE((SELECT MAX(sequence) FROM project_audit_events e WHERE e.project_code = p.code), 0) AS revision_sequence ",
                "FROM projects p WHERE p.code = ?1"
            ),
            params![code],
            stored_project_from_row,
        )
        .optional()
        .map_err(|error| AgentError::new("project_query_failed", error.to_string()))
}

pub(crate) fn load_projects(connection: &Connection) -> Result<Vec<StoredProject>, AgentError> {
    let mut statement = connection
        .prepare(
            concat!(
                "SELECT p.code, p.customer_name, p.stage, p.execution_mode, p.created_at, p.archived_at, ",
                "COALESCE((SELECT MAX(sequence) FROM project_audit_events e WHERE e.project_code = p.code), 0) AS revision_sequence ",
                "FROM projects p ORDER BY p.code"
            ),
        )
        .map_err(|error| AgentError::new("project_query_failed", error.to_string()))?;
    let rows = statement
        .query_map([], stored_project_from_row)
        .map_err(|error| AgentError::new("project_query_failed", error.to_string()))?;
    let mut projects = Vec::new();
    for row in rows {
        projects
            .push(row.map_err(|error| AgentError::new("project_query_failed", error.to_string()))?);
    }
    Ok(projects)
}

fn stored_project_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<StoredProject> {
    Ok(StoredProject {
        code: row.get(0)?,
        customer_name: row.get(1)?,
        stage: row.get(2)?,
        execution_mode: row.get(3)?,
        created_at: row.get(4)?,
        archived_at: row.get(5)?,
        revision_sequence: row.get(6)?,
    })
}

pub(crate) fn is_review_item_completed(
    connection: &Connection,
    code: &str,
    item: &str,
) -> Result<bool, AgentError> {
    let completed = connection
        .query_row(
            concat!(
                "SELECT completed FROM contract_review_items ",
                "WHERE project_code = ?1 AND item = ?2"
            ),
            params![code, item],
            |row| row.get::<_, u8>(0),
        )
        .optional()
        .map_err(|error| AgentError::new("contract_review_query_failed", error.to_string()))?;
    Ok(completed == Some(1))
}

pub(crate) fn existing_operation(
    connection: &Connection,
    operation_id: &str,
) -> Result<Option<StoredOperation>, AgentError> {
    connection
        .query_row(
            concat!(
                "SELECT operation_id, entity_id, operation_kind, base_revision, resulting_revision, ",
                "actor_id, device_id, correlation_id, payload_checksum ",
                "FROM sync_db.sync_operations WHERE operation_id = ?1"
            ),
            params![operation_id],
            |row| {
                Ok(StoredOperation {
                    operation_id: row.get(0)?,
                    entity_id: row.get(1)?,
                    operation_kind: row.get(2)?,
                    base_revision: row.get(3)?,
                    resulting_revision: row.get(4)?,
                    actor_id: row.get(5)?,
                    device_id: row.get(6)?,
                    correlation_id: row.get(7)?,
                    payload_checksum: row.get(8)?,
                })
            },
        )
        .optional()
        .map_err(|error| AgentError::new("sync_outbox_query_failed", error.to_string()))
}

pub(crate) fn ensure_operation_replay(
    operation: &StoredOperation,
    operation_id: &str,
    expected: OperationFingerprintInput<'_>,
) -> Result<(), AgentError> {
    let expected_fingerprint = operation_fingerprint(&expected);
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
        "operation_id is already used for a different canonical operation fingerprint",
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

pub(crate) fn next_audit_sequence(connection: &Connection, code: &str) -> Result<u64, AgentError> {
    connection
        .query_row(
            "SELECT COALESCE(MAX(sequence), 0) + 1 FROM project_audit_events WHERE project_code = ?1",
            params![code],
            |row| row.get(0),
        )
        .map_err(|error| AgentError::new("audit_query_failed", error.to_string()))
}

pub(crate) fn insert_audit_event(
    transaction: &Transaction<'_>,
    input: AuditEventInput<'_>,
) -> Result<(), AgentError> {
    transaction
        .execute(
            concat!(
                "INSERT INTO project_audit_events ",
                "(project_code, sequence, actor, action, reason, payload_json, occurred_at) ",
                "VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)"
            ),
            params![
                input.project_code,
                input.sequence,
                input.actor,
                input.action,
                input.reason,
                input.payload_json,
                input.timestamp
            ],
        )
        .map_err(|error| AgentError::new("audit_write_failed", error.to_string()))?;
    Ok(())
}

pub(crate) fn insert_sync_operation(
    transaction: &Transaction<'_>,
    input: SyncOperationInput<'_>,
) -> Result<(), AgentError> {
    let checksum = operation_fingerprint(&OperationFingerprintInput {
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
                "VALUES (?1, 'project_records', 'project', ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 'pending', ?11, ?11)"
            ),
            params![
                input.operation_id,
                input.entity_id,
                input.operation_kind,
                input.base_revision,
                input.resulting_revision,
                input.actor_id,
                input.device_id,
                input.correlation_id,
                input.payload_json,
                checksum,
                input.timestamp
            ],
        )
        .map_err(|error| AgentError::new("sync_outbox_write_failed", error.to_string()))?;
    Ok(())
}

fn operation_fingerprint(input: &OperationFingerprintInput<'_>) -> String {
    payload_checksum(&operation_fingerprint_json(input))
}

fn operation_fingerprint_json(input: &OperationFingerprintInput<'_>) -> String {
    let payload = serde_json::from_str::<serde_json::Value>(input.payload_json)
        .expect("canonical operation payload must be valid JSON");
    render_json(&json!({
        "domain": "project_records",
        "entity_type": "project",
        "entity_id": input.entity_id,
        "operation_kind": input.operation_kind,
        "base_revision": input.base_revision,
        "actor_id": input.actor_id,
        "device_id": input.device_id,
        "correlation_id": input.correlation_id,
        "payload": payload,
    }))
}

fn payload_checksum(payload_json: &str) -> String {
    let digest = Sha256::digest(payload_json.as_bytes());
    format!("sha256:{digest:x}")
}
