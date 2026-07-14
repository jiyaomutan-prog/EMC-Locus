use crate::{
    render_json,
    sqlite_policy::{enforce_project_slice_journal_mode, AttachedDatabase},
    AgentError,
};
use rusqlite::{params, Connection, OptionalExtension, Transaction};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::path::Path;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct StoredStationSetupIdentity {
    pub(crate) setup_id: String,
    pub(crate) label: String,
    pub(crate) current_ready_revision_id: Option<String>,
    pub(crate) created_by: String,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct StoredStationSetupRevision {
    pub(crate) revision_id: String,
    pub(crate) setup_id: String,
    pub(crate) revision_number: u32,
    pub(crate) parent_revision_id: Option<String>,
    pub(crate) status: String,
    pub(crate) definition_schema_version: String,
    pub(crate) definition_json: String,
    pub(crate) definition_checksum: String,
    pub(crate) readiness_json: String,
    pub(crate) created_by: String,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
    pub(crate) ready_at: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct StoredStationSetupAuditEvent {
    pub(crate) audit_id: u64,
    pub(crate) setup_id: String,
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
pub(crate) struct StoredStationSetupOperation {
    pub(crate) operation_id: String,
    pub(crate) setup_id: String,
    pub(crate) action: String,
    pub(crate) actor: String,
    pub(crate) device_id: String,
    pub(crate) correlation_id: String,
    pub(crate) payload_checksum: String,
    pub(crate) result_revision_id: String,
    pub(crate) result_definition_checksum: String,
}

pub(crate) struct NewStationSetupIdentity<'a> {
    pub(crate) setup_id: &'a str,
    pub(crate) label: &'a str,
    pub(crate) created_by: &'a str,
    pub(crate) timestamp: &'a str,
}

pub(crate) struct NewStationSetupRevision<'a> {
    pub(crate) revision_id: &'a str,
    pub(crate) setup_id: &'a str,
    pub(crate) revision_number: u32,
    pub(crate) parent_revision_id: Option<&'a str>,
    pub(crate) status: &'a str,
    pub(crate) definition_schema_version: &'a str,
    pub(crate) definition_json: &'a str,
    pub(crate) definition_checksum: &'a str,
    pub(crate) readiness_json: &'a str,
    pub(crate) created_by: &'a str,
    pub(crate) timestamp: &'a str,
}

pub(crate) struct ReplaceStationSetupDraft<'a> {
    pub(crate) setup_id: &'a str,
    pub(crate) revision_id: &'a str,
    pub(crate) expected_definition_checksum: &'a str,
    pub(crate) label: &'a str,
    pub(crate) definition_schema_version: &'a str,
    pub(crate) definition_json: &'a str,
    pub(crate) definition_checksum: &'a str,
    pub(crate) readiness_json: &'a str,
    pub(crate) timestamp: &'a str,
}

pub(crate) struct StationSetupAuditInput<'a> {
    pub(crate) setup_id: &'a str,
    pub(crate) revision_id: Option<&'a str>,
    pub(crate) action: &'a str,
    pub(crate) actor: &'a str,
    pub(crate) reason: &'a str,
    pub(crate) old_revision_id: Option<&'a str>,
    pub(crate) new_revision_id: Option<&'a str>,
    pub(crate) old_definition_checksum: Option<&'a str>,
    pub(crate) new_definition_checksum: Option<&'a str>,
    pub(crate) operation_id: &'a str,
    pub(crate) device_id: &'a str,
    pub(crate) correlation_id: &'a str,
    pub(crate) payload_json: &'a str,
    pub(crate) timestamp: &'a str,
}

pub(crate) struct StationSetupOperationInput<'a> {
    pub(crate) operation_id: &'a str,
    pub(crate) setup_id: &'a str,
    pub(crate) action: &'a str,
    pub(crate) actor: &'a str,
    pub(crate) device_id: &'a str,
    pub(crate) correlation_id: &'a str,
    pub(crate) payload_checksum: &'a str,
    pub(crate) result_revision_id: &'a str,
    pub(crate) result_definition_checksum: &'a str,
    pub(crate) timestamp: &'a str,
}

pub(crate) struct StationSetupOutboxInput<'a> {
    pub(crate) operation_id: &'a str,
    pub(crate) setup_id: &'a str,
    pub(crate) operation_kind: &'a str,
    pub(crate) base_revision: &'a str,
    pub(crate) resulting_revision: &'a str,
    pub(crate) actor: &'a str,
    pub(crate) device_id: &'a str,
    pub(crate) correlation_id: &'a str,
    pub(crate) payload_json: &'a str,
    pub(crate) timestamp: &'a str,
}

pub(crate) fn open_station_connection(storage_root: &Path) -> Result<Connection, AgentError> {
    let database = storage_root.join("station.sqlite");
    if !database.exists() {
        return Err(AgentError::new(
            "storage_not_initialized",
            "station setup reads require initialized station.sqlite",
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
    ensure_station_tables(&connection)?;
    Ok(connection)
}

pub(crate) fn open_station_connection_with_sync(
    storage_root: &Path,
) -> Result<Connection, AgentError> {
    let station_database = storage_root.join("station.sqlite");
    let sync_database = storage_root.join("sync.sqlite");
    if !station_database.exists() || !sync_database.exists() {
        return Err(AgentError::new(
            "storage_not_initialized",
            "station setup writes require initialized station.sqlite and sync.sqlite",
        ));
    }
    let connection = Connection::open(&station_database).map_err(|error| {
        AgentError::new(
            "database_open_error",
            format!("cannot open {}: {error}", station_database.display()),
        )
    })?;
    connection
        .execute_batch("PRAGMA foreign_keys = ON;")
        .map_err(|error| AgentError::new("database_pragma_error", error.to_string()))?;
    connection
        .execute(
            "ATTACH DATABASE ?1 AS sync_db",
            params![sync_database.to_string_lossy().to_string()],
        )
        .map_err(|error| AgentError::new("database_attach_error", error.to_string()))?;
    enforce_project_slice_journal_mode(&connection, AttachedDatabase::Main, "station.sqlite")?;
    enforce_project_slice_journal_mode(&connection, AttachedDatabase::SyncDb, "sync.sqlite")?;
    ensure_station_tables(&connection)?;
    if !table_exists(&connection, "sync_db", "sync_operations")? {
        return Err(AgentError::new(
            "storage_not_initialized",
            "missing required table sync_db.sync_operations",
        ));
    }
    Ok(connection)
}

fn ensure_station_tables(connection: &Connection) -> Result<(), AgentError> {
    for table in [
        "schema_migrations",
        "repository_metadata",
        "station_setup_identities",
        "station_setup_revisions",
        "station_setup_audit_events",
        "station_setup_operations",
    ] {
        if !table_exists(connection, "main", table)? {
            return Err(AgentError::new(
                "storage_not_initialized",
                format!("missing required station setup table {table}"),
            ));
        }
    }
    Ok(())
}

fn table_exists(connection: &Connection, schema: &str, table: &str) -> Result<bool, AgentError> {
    let sql = format!(
        "SELECT EXISTS(SELECT 1 FROM {schema}.sqlite_schema WHERE type = 'table' AND name = ?1)"
    );
    connection
        .query_row(&sql, params![table], |row| row.get::<_, bool>(0))
        .map_err(|error| AgentError::new("database_schema_query_failed", error.to_string()))
}

pub(crate) fn list_station_setup_identities(
    connection: &Connection,
) -> Result<Vec<StoredStationSetupIdentity>, AgentError> {
    let mut statement = connection
        .prepare(
            "SELECT setup_id, label, current_ready_revision_id, created_by, created_at, updated_at
             FROM station_setup_identities ORDER BY updated_at DESC, setup_id",
        )
        .map_err(|error| AgentError::new("station_setup_query_failed", error.to_string()))?;
    let rows = statement
        .query_map([], station_identity_from_row)
        .map_err(|error| AgentError::new("station_setup_query_failed", error.to_string()))?;
    collect_rows(rows, "station_setup_query_failed")
}

pub(crate) fn load_station_setup_identity(
    connection: &Connection,
    setup_id: &str,
) -> Result<Option<StoredStationSetupIdentity>, AgentError> {
    connection
        .query_row(
            "SELECT setup_id, label, current_ready_revision_id, created_by, created_at, updated_at
             FROM station_setup_identities WHERE setup_id = ?1",
            params![setup_id],
            station_identity_from_row,
        )
        .optional()
        .map_err(|error| AgentError::new("station_setup_query_failed", error.to_string()))
}

pub(crate) fn load_station_setup_revision(
    connection: &Connection,
    revision_id: &str,
) -> Result<Option<StoredStationSetupRevision>, AgentError> {
    connection
        .query_row(
            "SELECT revision_id, setup_id, revision_number, parent_revision_id, status,
                    definition_schema_version, definition_json, definition_checksum,
                    readiness_json, created_by, created_at, updated_at, ready_at
             FROM station_setup_revisions WHERE revision_id = ?1",
            params![revision_id],
            station_revision_from_row,
        )
        .optional()
        .map_err(|error| AgentError::new("station_setup_query_failed", error.to_string()))
}

pub(crate) fn load_station_setup_revisions(
    connection: &Connection,
    setup_id: &str,
) -> Result<Vec<StoredStationSetupRevision>, AgentError> {
    let mut statement = connection
        .prepare(
            "SELECT revision_id, setup_id, revision_number, parent_revision_id, status,
                    definition_schema_version, definition_json, definition_checksum,
                    readiness_json, created_by, created_at, updated_at, ready_at
             FROM station_setup_revisions WHERE setup_id = ?1 ORDER BY revision_number DESC",
        )
        .map_err(|error| AgentError::new("station_setup_query_failed", error.to_string()))?;
    let rows = statement
        .query_map(params![setup_id], station_revision_from_row)
        .map_err(|error| AgentError::new("station_setup_query_failed", error.to_string()))?;
    collect_rows(rows, "station_setup_query_failed")
}

pub(crate) fn load_active_station_setup_draft(
    connection: &Connection,
    setup_id: &str,
) -> Result<Option<StoredStationSetupRevision>, AgentError> {
    connection
        .query_row(
            "SELECT revision_id, setup_id, revision_number, parent_revision_id, status,
                    definition_schema_version, definition_json, definition_checksum,
                    readiness_json, created_by, created_at, updated_at, ready_at
             FROM station_setup_revisions WHERE setup_id = ?1 AND status = 'draft'",
            params![setup_id],
            station_revision_from_row,
        )
        .optional()
        .map_err(|error| AgentError::new("station_setup_query_failed", error.to_string()))
}

pub(crate) fn next_station_setup_revision_number(
    connection: &Connection,
    setup_id: &str,
) -> Result<u32, AgentError> {
    connection
        .query_row(
            "SELECT COALESCE(MAX(revision_number), 0) + 1 FROM station_setup_revisions WHERE setup_id = ?1",
            params![setup_id],
            |row| row.get(0),
        )
        .map_err(|error| AgentError::new("station_setup_query_failed", error.to_string()))
}

pub(crate) fn insert_station_setup_identity(
    transaction: &Transaction<'_>,
    input: NewStationSetupIdentity<'_>,
) -> Result<(), AgentError> {
    transaction
        .execute(
            "INSERT INTO station_setup_identities
                (setup_id, label, created_by, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?4)",
            params![
                input.setup_id,
                input.label,
                input.created_by,
                input.timestamp
            ],
        )
        .map_err(|error| AgentError::new("station_setup_write_failed", error.to_string()))?;
    Ok(())
}

pub(crate) fn insert_station_setup_revision(
    transaction: &Transaction<'_>,
    input: NewStationSetupRevision<'_>,
) -> Result<(), AgentError> {
    transaction
        .execute(
            "INSERT INTO station_setup_revisions (
                revision_id, setup_id, revision_number, parent_revision_id, status,
                definition_schema_version, definition_json, definition_checksum,
                readiness_json, created_by, created_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?11)",
            params![
                input.revision_id,
                input.setup_id,
                input.revision_number,
                input.parent_revision_id,
                input.status,
                input.definition_schema_version,
                input.definition_json,
                input.definition_checksum,
                input.readiness_json,
                input.created_by,
                input.timestamp
            ],
        )
        .map_err(|error| AgentError::new("station_setup_write_failed", error.to_string()))?;
    Ok(())
}

pub(crate) fn replace_station_setup_draft(
    transaction: &Transaction<'_>,
    input: ReplaceStationSetupDraft<'_>,
) -> Result<bool, AgentError> {
    let changed = transaction
        .execute(
            "UPDATE station_setup_revisions
             SET definition_schema_version = ?4, definition_json = ?5,
                 definition_checksum = ?6, readiness_json = ?7, updated_at = ?8
             WHERE setup_id = ?1 AND revision_id = ?2 AND status = 'draft'
               AND definition_checksum = ?3",
            params![
                input.setup_id,
                input.revision_id,
                input.expected_definition_checksum,
                input.definition_schema_version,
                input.definition_json,
                input.definition_checksum,
                input.readiness_json,
                input.timestamp
            ],
        )
        .map_err(|error| AgentError::new("station_setup_write_failed", error.to_string()))?;
    if changed == 1 {
        transaction
            .execute(
                "UPDATE station_setup_identities SET label = ?2, updated_at = ?3 WHERE setup_id = ?1",
                params![input.setup_id, input.label, input.timestamp],
            )
            .map_err(|error| AgentError::new("station_setup_write_failed", error.to_string()))?;
    }
    Ok(changed == 1)
}

pub(crate) fn mark_station_setup_ready(
    transaction: &Transaction<'_>,
    setup_id: &str,
    revision_id: &str,
    timestamp: &str,
) -> Result<(), AgentError> {
    transaction
        .execute(
            "UPDATE station_setup_revisions
             SET status = 'superseded'
             WHERE revision_id = (
                 SELECT current_ready_revision_id FROM station_setup_identities WHERE setup_id = ?1
             ) AND revision_id <> ?2 AND status = 'ready'",
            params![setup_id, revision_id],
        )
        .map_err(|error| AgentError::new("station_setup_write_failed", error.to_string()))?;
    let changed = transaction
        .execute(
            "UPDATE station_setup_revisions
             SET status = 'ready', ready_at = ?3, updated_at = ?3
             WHERE setup_id = ?1 AND revision_id = ?2 AND status = 'draft'",
            params![setup_id, revision_id, timestamp],
        )
        .map_err(|error| AgentError::new("station_setup_write_failed", error.to_string()))?;
    if changed != 1 {
        return Err(AgentError::new(
            "station_setup_revision_not_editable",
            "only a draft setup revision can be marked ready",
        ));
    }
    transaction
        .execute(
            "UPDATE station_setup_identities
             SET current_ready_revision_id = ?2, updated_at = ?3 WHERE setup_id = ?1",
            params![setup_id, revision_id, timestamp],
        )
        .map_err(|error| AgentError::new("station_setup_write_failed", error.to_string()))?;
    Ok(())
}

pub(crate) fn load_station_setup_operation(
    connection: &Connection,
    operation_id: &str,
) -> Result<Option<StoredStationSetupOperation>, AgentError> {
    connection
        .query_row(
            "SELECT operation_id, setup_id, action, actor, device_id, correlation_id,
                    payload_checksum, result_revision_id, result_definition_checksum
             FROM station_setup_operations WHERE operation_id = ?1",
            params![operation_id],
            |row| {
                Ok(StoredStationSetupOperation {
                    operation_id: row.get(0)?,
                    setup_id: row.get(1)?,
                    action: row.get(2)?,
                    actor: row.get(3)?,
                    device_id: row.get(4)?,
                    correlation_id: row.get(5)?,
                    payload_checksum: row.get(6)?,
                    result_revision_id: row.get(7)?,
                    result_definition_checksum: row.get(8)?,
                })
            },
        )
        .optional()
        .map_err(|error| AgentError::new("station_setup_query_failed", error.to_string()))
}

pub(crate) fn insert_station_setup_operation(
    transaction: &Transaction<'_>,
    input: StationSetupOperationInput<'_>,
) -> Result<(), AgentError> {
    transaction
        .execute(
            "INSERT INTO station_setup_operations (
                operation_id, setup_id, action, actor, device_id, correlation_id,
                payload_checksum, result_revision_id, result_definition_checksum, occurred_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                input.operation_id,
                input.setup_id,
                input.action,
                input.actor,
                input.device_id,
                input.correlation_id,
                input.payload_checksum,
                input.result_revision_id,
                input.result_definition_checksum,
                input.timestamp
            ],
        )
        .map_err(|error| AgentError::new("station_setup_write_failed", error.to_string()))?;
    Ok(())
}

pub(crate) fn insert_station_setup_audit_event(
    transaction: &Transaction<'_>,
    input: StationSetupAuditInput<'_>,
) -> Result<(), AgentError> {
    let payload_checksum = sha256_text(input.payload_json);
    transaction
        .execute(
            "INSERT INTO station_setup_audit_events (
                setup_id, revision_id, action, actor, reason, old_revision_id,
                new_revision_id, old_definition_checksum, new_definition_checksum,
                operation_id, device_id, correlation_id, payload_json, payload_checksum, occurred_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            params![
                input.setup_id,
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
                payload_checksum,
                input.timestamp
            ],
        )
        .map_err(|error| AgentError::new("station_setup_audit_write_failed", error.to_string()))?;
    Ok(())
}

pub(crate) fn load_station_setup_audit_events(
    connection: &Connection,
    setup_id: &str,
) -> Result<Vec<StoredStationSetupAuditEvent>, AgentError> {
    let mut statement = connection
        .prepare(
            "SELECT audit_id, setup_id, revision_id, action, actor, reason,
                    old_revision_id, new_revision_id, old_definition_checksum,
                    new_definition_checksum, operation_id, device_id, correlation_id,
                    payload_json, occurred_at
             FROM station_setup_audit_events WHERE setup_id = ?1 ORDER BY audit_id",
        )
        .map_err(|error| AgentError::new("station_setup_audit_query_failed", error.to_string()))?;
    let rows = statement
        .query_map(params![setup_id], |row| {
            Ok(StoredStationSetupAuditEvent {
                audit_id: row.get(0)?,
                setup_id: row.get(1)?,
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
        .map_err(|error| AgentError::new("station_setup_audit_query_failed", error.to_string()))?;
    collect_rows(rows, "station_setup_audit_query_failed")
}

pub(crate) fn insert_station_setup_outbox(
    transaction: &Transaction<'_>,
    input: StationSetupOutboxInput<'_>,
) -> Result<(), AgentError> {
    let payload_value: serde_json::Value = serde_json::from_str(input.payload_json)
        .unwrap_or_else(|_| json!({ "raw": input.payload_json }));
    let payload = render_json(&json!({
        "domain": "station_configurations",
        "entity_type": "station_measurement_setup",
        "entity_id": input.setup_id,
        "operation_kind": input.operation_kind,
        "payload": payload_value
    }));
    let payload_checksum = sha256_text(&payload);
    transaction
        .execute(
            "INSERT INTO sync_db.sync_operations (
                operation_id, domain, entity_type, entity_id, operation_kind,
                base_revision, resulting_revision, actor_id, device_id, correlation_id,
                payload_json, payload_checksum, status, occurred_at, recorded_at
             ) VALUES (?1, 'station_configurations', 'station_measurement_setup', ?2, ?3,
                       ?4, ?5, ?6, ?7, ?8, ?9, ?10, 'pending', ?11, ?11)",
            params![
                input.operation_id,
                input.setup_id,
                input.operation_kind,
                input.base_revision,
                input.resulting_revision,
                input.actor,
                input.device_id,
                input.correlation_id,
                payload,
                payload_checksum,
                input.timestamp
            ],
        )
        .map_err(|error| AgentError::new("station_setup_outbox_write_failed", error.to_string()))?;
    Ok(())
}

fn station_identity_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<StoredStationSetupIdentity> {
    Ok(StoredStationSetupIdentity {
        setup_id: row.get(0)?,
        label: row.get(1)?,
        current_ready_revision_id: row.get(2)?,
        created_by: row.get(3)?,
        created_at: row.get(4)?,
        updated_at: row.get(5)?,
    })
}

fn station_revision_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<StoredStationSetupRevision> {
    Ok(StoredStationSetupRevision {
        revision_id: row.get(0)?,
        setup_id: row.get(1)?,
        revision_number: row.get(2)?,
        parent_revision_id: row.get(3)?,
        status: row.get(4)?,
        definition_schema_version: row.get(5)?,
        definition_json: row.get(6)?,
        definition_checksum: row.get(7)?,
        readiness_json: row.get(8)?,
        created_by: row.get(9)?,
        created_at: row.get(10)?,
        updated_at: row.get(11)?,
        ready_at: row.get(12)?,
    })
}

fn collect_rows<T>(
    rows: rusqlite::MappedRows<'_, impl FnMut(&rusqlite::Row<'_>) -> rusqlite::Result<T>>,
    code: &'static str,
) -> Result<Vec<T>, AgentError> {
    let mut values = Vec::new();
    for row in rows {
        values.push(row.map_err(|error| AgentError::new(code, error.to_string()))?);
    }
    Ok(values)
}

pub(crate) fn sha256_text(value: &str) -> String {
    let digest = Sha256::digest(value.as_bytes());
    format!("sha256:{digest:x}")
}
