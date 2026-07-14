use crate::{
    render_json,
    sqlite_policy::{enforce_project_slice_journal_mode, AttachedDatabase},
    AgentError, AGENT_NAME,
};
use rusqlite::{params, Connection, OptionalExtension, Transaction};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::path::Path;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoredInstrument {
    pub asset_id: String,
    pub family: String,
    pub manufacturer: String,
    pub model: String,
    pub serial_number: String,
    pub availability: String,
    pub calibration_requirement: String,
    pub capabilities_json: String,
    pub category_code: Option<String>,
    pub equipment_model_id: Option<String>,
    pub equipment_model_revision_id: Option<String>,
    pub equipment_model_checksum: Option<String>,
    pub part_number: Option<String>,
    pub calibration_period_months: Option<u32>,
    pub calibration_due_warning_days: u32,
    pub metrology_notes: String,
    pub serviceability_status: String,
    pub serviceability_reason: String,
    pub serviceability_updated_at: Option<String>,
    pub legacy_availability: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub revision: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoredCalibrationRecord {
    pub id: i64,
    pub asset_id: String,
    pub certificate_reference: String,
    pub calibrated_at: String,
    pub due_at: String,
    pub provider: String,
    pub status_at_import: String,
    pub uncertainty_json: String,
    pub file_reference: Option<String>,
    pub checksum: Option<String>,
    pub created_at: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoredCalibrationEvent {
    pub event_id: String,
    pub asset_id: String,
    pub certificate_reference: String,
    pub calibrated_at: String,
    pub due_at: String,
    pub provider: String,
    pub decision: String,
    pub as_found_status: Option<String>,
    pub as_left_status: Option<String>,
    pub adjustment_performed: bool,
    pub uncertainty_summary_json: String,
    pub traceability_reference: Option<String>,
    pub comment: String,
    pub document_manifest_json: Option<String>,
    pub recorded_at: String,
    pub recorded_by: String,
    pub revision: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoredMetrologyOperation {
    pub operation_id: String,
    pub entity_id: String,
    pub operation_kind: String,
    pub base_revision: String,
    pub actor_id: String,
    pub device_id: String,
    pub correlation_id: String,
    pub payload_checksum: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoredMetrologyAuditEvent {
    pub sequence: u64,
    pub actor: String,
    pub action: String,
    pub reason: String,
    pub operation_id: String,
    pub correlation_id: String,
    pub device_id: String,
    pub base_revision: String,
    pub resulting_revision: String,
    pub payload_json: String,
    pub occurred_at: String,
}

pub struct NewInstrumentRecord<'a> {
    pub asset_id: &'a str,
    pub family: &'a str,
    pub manufacturer: &'a str,
    pub model: &'a str,
    pub serial_number: &'a str,
    pub category_code: Option<&'a str>,
    pub equipment_model_id: Option<&'a str>,
    pub equipment_model_revision_id: Option<&'a str>,
    pub equipment_model_checksum: Option<&'a str>,
    pub part_number: Option<&'a str>,
    pub calibration_requirement: &'a str,
    pub calibration_period_months: Option<u32>,
    pub calibration_due_warning_days: u32,
    pub capabilities_json: &'a str,
    pub metrology_notes: &'a str,
    pub serviceability_status: &'a str,
    pub serviceability_reason: &'a str,
    pub timestamp: &'a str,
}

pub struct NewCalibrationEventRecord<'a> {
    pub event_id: &'a str,
    pub asset_id: &'a str,
    pub certificate_reference: &'a str,
    pub calibrated_at: &'a str,
    pub due_at: &'a str,
    pub provider: &'a str,
    pub decision: &'a str,
    pub as_found_status: Option<&'a str>,
    pub as_left_status: Option<&'a str>,
    pub adjustment_performed: bool,
    pub uncertainty_summary_json: &'a str,
    pub traceability_reference: Option<&'a str>,
    pub comment: &'a str,
    pub document_manifest_json: Option<&'a str>,
    pub recorded_at: &'a str,
    pub recorded_by: &'a str,
    pub revision: &'a str,
}

pub struct MetrologyOperationFingerprintInput<'a> {
    pub entity_type: &'a str,
    pub entity_id: &'a str,
    pub operation_kind: &'a str,
    pub base_revision: &'a str,
    pub actor_id: &'a str,
    pub device_id: &'a str,
    pub correlation_id: &'a str,
    pub payload_json: &'a str,
}

pub struct MetrologyAuditEventInput<'a> {
    pub entity_type: &'a str,
    pub entity_id: &'a str,
    pub sequence: u64,
    pub actor: &'a str,
    pub action: &'a str,
    pub reason: &'a str,
    pub operation_id: &'a str,
    pub correlation_id: &'a str,
    pub device_id: &'a str,
    pub base_revision: &'a str,
    pub resulting_revision: &'a str,
    pub payload_json: &'a str,
    pub timestamp: &'a str,
}

pub struct MetrologySyncOperationInput<'a> {
    pub operation_id: &'a str,
    pub entity_type: &'a str,
    pub entity_id: &'a str,
    pub operation_kind: &'a str,
    pub base_revision: &'a str,
    pub resulting_revision: &'a str,
    pub actor_id: &'a str,
    pub device_id: &'a str,
    pub correlation_id: &'a str,
    pub payload_json: &'a str,
    pub timestamp: &'a str,
}

pub fn open_metrology_connection(storage_root: &Path) -> Result<Connection, AgentError> {
    let database = storage_root.join("metrology.sqlite");
    if !database.exists() {
        return Err(AgentError::new(
            "storage_not_initialized",
            "metrology commands require initialized metrology.sqlite",
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
    ensure_metrology_tables(&connection)?;
    Ok(connection)
}

pub fn open_metrology_connection_with_sync(storage_root: &Path) -> Result<Connection, AgentError> {
    let metrology_database = storage_root.join("metrology.sqlite");
    let sync_database = storage_root.join("sync.sqlite");
    if !metrology_database.exists() || !sync_database.exists() {
        return Err(AgentError::new(
            "storage_not_initialized",
            "metrology writes require initialized metrology.sqlite and sync.sqlite",
        ));
    }
    let connection = Connection::open(&metrology_database).map_err(|error| {
        AgentError::new(
            "database_open_error",
            format!("cannot open {}: {error}", metrology_database.display()),
        )
    })?;
    connection
        .execute_batch("PRAGMA foreign_keys = ON;")
        .map_err(|error| AgentError::new("database_pragma_error", error.to_string()))?;
    let sync_path = sync_database.to_string_lossy().to_string();
    connection
        .execute("ATTACH DATABASE ?1 AS sync_db", params![sync_path])
        .map_err(|error| AgentError::new("database_attach_error", error.to_string()))?;
    enforce_project_slice_journal_mode(&connection, AttachedDatabase::Main, "metrology.sqlite")?;
    enforce_project_slice_journal_mode(&connection, AttachedDatabase::SyncDb, "sync.sqlite")?;
    ensure_metrology_tables(&connection)?;
    ensure_sync_tables(&connection)?;
    Ok(connection)
}

fn ensure_metrology_tables(connection: &Connection) -> Result<(), AgentError> {
    for table in [
        "schema_migrations",
        "repository_metadata",
        "instrument_categories",
        "instruments",
        "calibration_records",
        "calibration_events",
        "instrument_documents",
        "metrology_audit_events",
    ] {
        if !table_exists(connection, table)? {
            return Err(AgentError::new(
                "storage_not_initialized",
                format!("missing required metrology table {table}"),
            ));
        }
    }

    for column in [
        "serviceability_status",
        "serviceability_reason",
        "serviceability_updated_at",
        "legacy_availability",
        "calibration_due_warning_days",
    ] {
        if !column_exists(connection, "instruments", column)? {
            return Err(AgentError::new(
                "metrology_schema_outdated",
                format!("missing metrology instruments column {column}"),
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

fn table_exists(connection: &Connection, table: &str) -> Result<bool, AgentError> {
    table_exists_in_schema(connection, "main", table)
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

fn column_exists(connection: &Connection, table: &str, column: &str) -> Result<bool, AgentError> {
    let mut statement = connection
        .prepare(&format!("PRAGMA table_info({table})"))
        .map_err(|error| AgentError::new("database_invalid", error.to_string()))?;
    let rows = statement
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|error| AgentError::new("database_invalid", error.to_string()))?;
    for row in rows {
        if row.map_err(|error| AgentError::new("database_invalid", error.to_string()))? == column {
            return Ok(true);
        }
    }
    Ok(false)
}

pub fn load_instrument(
    connection: &Connection,
    asset_id: &str,
) -> Result<Option<StoredInstrument>, AgentError> {
    connection
        .query_row(
            instrument_select_sql("WHERE asset_id = ?1").as_str(),
            params![asset_id],
            stored_instrument_from_row,
        )
        .optional()
        .map_err(|error| AgentError::new("metrology_instrument_query_failed", error.to_string()))
}

pub fn load_instruments(connection: &Connection) -> Result<Vec<StoredInstrument>, AgentError> {
    let sql = instrument_select_sql("ORDER BY asset_id");
    let mut statement = connection
        .prepare(&sql)
        .map_err(|error| AgentError::new("metrology_instrument_query_failed", error.to_string()))?;
    let rows = statement
        .query_map([], stored_instrument_from_row)
        .map_err(|error| AgentError::new("metrology_instrument_query_failed", error.to_string()))?;
    let mut instruments = Vec::new();
    for row in rows {
        instruments.push(row.map_err(|error| {
            AgentError::new("metrology_instrument_query_failed", error.to_string())
        })?);
    }
    Ok(instruments)
}

fn instrument_select_sql(suffix: &str) -> String {
    let base = concat!(
        "SELECT asset_id, family, manufacturer, model, serial_number, availability, ",
        "calibration_requirement, capabilities_json, category_code, part_number, ",
        "calibration_period_months, metrology_notes, serviceability_status, ",
        "serviceability_reason, serviceability_updated_at, legacy_availability, ",
        "calibration_due_warning_days, equipment_model_id, equipment_model_revision_id, ",
        "equipment_model_checksum, ",
        "created_at, updated_at FROM instruments "
    );
    format!("{base}{suffix}")
}

fn stored_instrument_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<StoredInstrument> {
    let asset_id: String = row.get(0)?;
    let updated_at: String = row.get(21)?;
    Ok(StoredInstrument {
        revision: revision_for("instrument", &asset_id, &updated_at),
        asset_id,
        family: row.get(1)?,
        manufacturer: row.get(2)?,
        model: row.get(3)?,
        serial_number: row.get(4)?,
        availability: row.get(5)?,
        calibration_requirement: row.get(6)?,
        capabilities_json: row.get(7)?,
        category_code: row.get(8)?,
        part_number: row.get(9)?,
        calibration_period_months: row.get(10)?,
        metrology_notes: row.get(11)?,
        serviceability_status: row.get(12)?,
        serviceability_reason: row.get(13)?,
        serviceability_updated_at: row.get(14)?,
        legacy_availability: row.get(15)?,
        calibration_due_warning_days: row.get(16)?,
        equipment_model_id: row.get(17)?,
        equipment_model_revision_id: row.get(18)?,
        equipment_model_checksum: row.get(19)?,
        created_at: row.get(20)?,
        updated_at,
    })
}

pub fn load_latest_calibration_record(
    connection: &Connection,
    asset_id: &str,
) -> Result<Option<StoredCalibrationRecord>, AgentError> {
    connection
        .query_row(
            concat!(
                "SELECT id, asset_id, certificate_reference, calibrated_at, due_at, provider, ",
                "status_at_import, uncertainty_json, file_reference, checksum, created_at ",
                "FROM calibration_records WHERE asset_id = ?1 ",
                "ORDER BY due_at DESC, calibrated_at DESC, id DESC LIMIT 1"
            ),
            params![asset_id],
            |row| {
                Ok(StoredCalibrationRecord {
                    id: row.get(0)?,
                    asset_id: row.get(1)?,
                    certificate_reference: row.get(2)?,
                    calibrated_at: row.get(3)?,
                    due_at: row.get(4)?,
                    provider: row.get(5)?,
                    status_at_import: row.get(6)?,
                    uncertainty_json: row.get(7)?,
                    file_reference: row.get(8)?,
                    checksum: row.get(9)?,
                    created_at: row.get(10)?,
                })
            },
        )
        .optional()
        .map_err(|error| AgentError::new("metrology_calibration_query_failed", error.to_string()))
}

pub fn load_calibration_events(
    connection: &Connection,
    asset_id: &str,
) -> Result<Vec<StoredCalibrationEvent>, AgentError> {
    let mut statement = connection
        .prepare(concat!(
            "SELECT event_id, asset_id, certificate_reference, calibrated_at, due_at, ",
            "provider, decision, as_found_status, as_left_status, adjustment_performed, ",
            "uncertainty_summary_json, traceability_reference, comment, ",
            "document_manifest_json, recorded_at, recorded_by, revision ",
            "FROM calibration_events WHERE asset_id = ?1 ",
            "ORDER BY due_at DESC, calibrated_at DESC, event_id DESC"
        ))
        .map_err(|error| {
            AgentError::new("metrology_calibration_query_failed", error.to_string())
        })?;
    let rows = statement
        .query_map(params![asset_id], stored_calibration_event_from_row)
        .map_err(|error| {
            AgentError::new("metrology_calibration_query_failed", error.to_string())
        })?;
    let mut events = Vec::new();
    for row in rows {
        events.push(row.map_err(|error| {
            AgentError::new("metrology_calibration_query_failed", error.to_string())
        })?);
    }
    Ok(events)
}

pub fn load_latest_calibration_event(
    connection: &Connection,
    asset_id: &str,
) -> Result<Option<StoredCalibrationEvent>, AgentError> {
    connection
        .query_row(
            concat!(
                "SELECT event_id, asset_id, certificate_reference, calibrated_at, due_at, ",
                "provider, decision, as_found_status, as_left_status, adjustment_performed, ",
                "uncertainty_summary_json, traceability_reference, comment, ",
                "document_manifest_json, recorded_at, recorded_by, revision ",
                "FROM calibration_events WHERE asset_id = ?1 ",
                "ORDER BY due_at DESC, calibrated_at DESC, event_id DESC LIMIT 1"
            ),
            params![asset_id],
            stored_calibration_event_from_row,
        )
        .optional()
        .map_err(|error| AgentError::new("metrology_calibration_query_failed", error.to_string()))
}

pub fn load_metrology_audit_events(
    connection: &Connection,
    entity_type: &str,
    entity_id: &str,
) -> Result<Vec<StoredMetrologyAuditEvent>, AgentError> {
    let mut statement = connection
        .prepare(concat!(
            "SELECT sequence, actor, action, reason, operation_id, correlation_id, ",
            "device_id, base_revision, resulting_revision, payload_json, occurred_at ",
            "FROM metrology_audit_events WHERE entity_type = ?1 AND entity_id = ?2 ",
            "ORDER BY sequence"
        ))
        .map_err(|error| AgentError::new("metrology_audit_query_failed", error.to_string()))?;
    let rows = statement
        .query_map(params![entity_type, entity_id], |row| {
            Ok(StoredMetrologyAuditEvent {
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
        .map_err(|error| AgentError::new("metrology_audit_query_failed", error.to_string()))?;
    let mut events = Vec::new();
    for row in rows {
        events.push(
            row.map_err(|error| {
                AgentError::new("metrology_audit_query_failed", error.to_string())
            })?,
        );
    }
    Ok(events)
}

pub fn load_calibration_event(
    connection: &Connection,
    event_id: &str,
) -> Result<Option<StoredCalibrationEvent>, AgentError> {
    connection
        .query_row(
            concat!(
                "SELECT event_id, asset_id, certificate_reference, calibrated_at, due_at, ",
                "provider, decision, as_found_status, as_left_status, adjustment_performed, ",
                "uncertainty_summary_json, traceability_reference, comment, ",
                "document_manifest_json, recorded_at, recorded_by, revision ",
                "FROM calibration_events WHERE event_id = ?1"
            ),
            params![event_id],
            stored_calibration_event_from_row,
        )
        .optional()
        .map_err(|error| AgentError::new("metrology_calibration_query_failed", error.to_string()))
}

fn stored_calibration_event_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<StoredCalibrationEvent> {
    Ok(StoredCalibrationEvent {
        event_id: row.get(0)?,
        asset_id: row.get(1)?,
        certificate_reference: row.get(2)?,
        calibrated_at: row.get(3)?,
        due_at: row.get(4)?,
        provider: row.get(5)?,
        decision: row.get(6)?,
        as_found_status: row.get(7)?,
        as_left_status: row.get(8)?,
        adjustment_performed: row.get::<_, u8>(9)? == 1,
        uncertainty_summary_json: row.get(10)?,
        traceability_reference: row.get(11)?,
        comment: row.get(12)?,
        document_manifest_json: row.get(13)?,
        recorded_at: row.get(14)?,
        recorded_by: row.get(15)?,
        revision: row.get(16)?,
    })
}

pub fn insert_instrument(
    connection: &Connection,
    input: NewInstrumentRecord<'_>,
) -> Result<(), AgentError> {
    connection
        .execute(
            concat!(
                "INSERT INTO instruments (asset_id, family, manufacturer, model, serial_number, ",
                "availability, calibration_requirement, capabilities_json, category_code, ",
                "part_number, calibration_period_months, calibration_due_warning_days, ",
                "metrology_notes, serviceability_status, equipment_model_id, ",
                "equipment_model_revision_id, equipment_model_checksum, ",
                "serviceability_reason, serviceability_updated_at, legacy_availability, created_at, updated_at) ",
                "VALUES (?1, ?2, ?3, ?4, ?5, 'available', ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, 'available', ?18, ?18)"
            ),
            params![
                input.asset_id,
                input.family,
                input.manufacturer,
                input.model,
                input.serial_number,
                input.calibration_requirement,
                input.capabilities_json,
                input.category_code,
                input.part_number,
                input.calibration_period_months,
                input.calibration_due_warning_days,
                input.metrology_notes,
                input.serviceability_status,
                input.equipment_model_id,
                input.equipment_model_revision_id,
                input.equipment_model_checksum,
                input.serviceability_reason,
                input.timestamp,
            ],
        )
        .map_err(|error| AgentError::new("metrology_instrument_write_failed", error.to_string()))?;
    Ok(())
}

pub fn insert_calibration_event(
    connection: &Connection,
    input: NewCalibrationEventRecord<'_>,
) -> Result<(), AgentError> {
    connection
        .execute(
            concat!(
                "INSERT INTO calibration_events (event_id, asset_id, certificate_reference, ",
                "calibrated_at, due_at, provider, decision, as_found_status, as_left_status, ",
                "adjustment_performed, uncertainty_summary_json, traceability_reference, ",
                "comment, document_manifest_json, recorded_at, recorded_by, revision) ",
                "VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)"
            ),
            params![
                input.event_id,
                input.asset_id,
                input.certificate_reference,
                input.calibrated_at,
                input.due_at,
                input.provider,
                input.decision,
                input.as_found_status,
                input.as_left_status,
                if input.adjustment_performed { 1u8 } else { 0u8 },
                input.uncertainty_summary_json,
                input.traceability_reference,
                input.comment,
                input.document_manifest_json,
                input.recorded_at,
                input.recorded_by,
                input.revision,
            ],
        )
        .map_err(|error| AgentError::new("metrology_calibration_write_failed", error.to_string()))?;
    Ok(())
}

pub fn update_instrument_serviceability(
    transaction: &Transaction<'_>,
    asset_id: &str,
    serviceability_status: &str,
    serviceability_reason: &str,
    timestamp: &str,
) -> Result<(), AgentError> {
    transaction
        .execute(
            concat!(
                "UPDATE instruments SET serviceability_status = ?2, serviceability_reason = ?3, ",
                "serviceability_updated_at = ?4, updated_at = ?4 WHERE asset_id = ?1"
            ),
            params![
                asset_id,
                serviceability_status,
                serviceability_reason,
                timestamp,
            ],
        )
        .map_err(|error| AgentError::new("metrology_instrument_write_failed", error.to_string()))?;
    Ok(())
}

pub fn existing_metrology_operation(
    connection: &Connection,
    operation_id: &str,
) -> Result<Option<StoredMetrologyOperation>, AgentError> {
    connection
        .query_row(
            concat!(
                "SELECT operation_id, entity_type, entity_id, action, base_revision, ",
                "actor, device_id, correlation_id, payload_checksum ",
                "FROM metrology_audit_events WHERE operation_id = ?1"
            ),
            params![operation_id],
            |row| {
                let entity_type: String = row.get(1)?;
                let entity_id: String = row.get(2)?;
                Ok(StoredMetrologyOperation {
                    operation_id: row.get(0)?,
                    entity_id: format!("{entity_type}:{entity_id}"),
                    operation_kind: row.get(3)?,
                    base_revision: row.get(4)?,
                    actor_id: row.get(5)?,
                    device_id: row.get(6)?,
                    correlation_id: row.get(7)?,
                    payload_checksum: row.get(8)?,
                })
            },
        )
        .optional()
        .map_err(|error| AgentError::new("metrology_audit_query_failed", error.to_string()))
}

pub fn ensure_metrology_operation_replay(
    operation: &StoredMetrologyOperation,
    operation_id: &str,
    expected: MetrologyOperationFingerprintInput<'_>,
) -> Result<(), AgentError> {
    let expected_fingerprint = metrology_operation_fingerprint(&expected);
    let expected_entity = format!("{}:{}", expected.entity_type, expected.entity_id);
    if operation.entity_id == expected_entity
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
        "operation_id is already used for a different canonical metrology operation fingerprint",
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

pub fn next_metrology_audit_sequence(
    connection: &Connection,
    entity_type: &str,
    entity_id: &str,
) -> Result<u64, AgentError> {
    connection
        .query_row(
            concat!(
                "SELECT COALESCE(MAX(sequence), 0) + 1 FROM metrology_audit_events ",
                "WHERE entity_type = ?1 AND entity_id = ?2"
            ),
            params![entity_type, entity_id],
            |row| row.get(0),
        )
        .map_err(|error| AgentError::new("metrology_audit_query_failed", error.to_string()))
}

pub fn insert_metrology_audit_event(
    transaction: &Transaction<'_>,
    input: MetrologyAuditEventInput<'_>,
) -> Result<(), AgentError> {
    let checksum = metrology_operation_fingerprint(&MetrologyOperationFingerprintInput {
        entity_type: input.entity_type,
        entity_id: input.entity_id,
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
                "INSERT INTO metrology_audit_events ",
                "(entity_type, entity_id, sequence, actor, action, reason, operation_id, ",
                "correlation_id, device_id, base_revision, resulting_revision, payload_json, ",
                "payload_checksum, occurred_at) ",
                "VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)"
            ),
            params![
                input.entity_type,
                input.entity_id,
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
        .map_err(|error| AgentError::new("metrology_audit_write_failed", error.to_string()))?;
    Ok(())
}

pub fn insert_metrology_sync_operation(
    transaction: &Transaction<'_>,
    input: MetrologySyncOperationInput<'_>,
) -> Result<(), AgentError> {
    let checksum = metrology_operation_fingerprint(&MetrologyOperationFingerprintInput {
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
                "VALUES (?1, 'metrology', ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, 'pending', ?12, ?12)"
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

fn metrology_operation_fingerprint(input: &MetrologyOperationFingerprintInput<'_>) -> String {
    payload_checksum(&render_json(&json!({
        "domain": "metrology",
        "entity_type": input.entity_type,
        "entity_id": input.entity_id,
        "operation_kind": input.operation_kind,
        "base_revision": input.base_revision,
        "actor_id": input.actor_id,
        "device_id": input.device_id,
        "correlation_id": input.correlation_id,
        "payload": serde_json::from_str::<serde_json::Value>(input.payload_json)
            .expect("canonical metrology operation payload must be valid JSON"),
    })))
}

fn payload_checksum(payload_json: &str) -> String {
    let digest = Sha256::digest(payload_json.as_bytes());
    format!("sha256:{digest:x}")
}

fn revision_for(entity_type: &str, entity_id: &str, updated_at: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(AGENT_NAME.as_bytes());
    hasher.update(b":");
    hasher.update(entity_type.as_bytes());
    hasher.update(b":");
    hasher.update(entity_id.as_bytes());
    hasher.update(b":");
    hasher.update(updated_at.as_bytes());
    let digest = format!("{:x}", hasher.finalize());
    format!("rev-{}", &digest[..12])
}
