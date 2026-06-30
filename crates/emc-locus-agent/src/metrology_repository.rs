use crate::{AgentError, AGENT_NAME};
use rusqlite::{params, Connection, OptionalExtension};
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

pub struct NewInstrumentRecord<'a> {
    pub asset_id: &'a str,
    pub family: &'a str,
    pub manufacturer: &'a str,
    pub model: &'a str,
    pub serial_number: &'a str,
    pub category_code: &'a str,
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

fn ensure_metrology_tables(connection: &Connection) -> Result<(), AgentError> {
    for table in [
        "schema_migrations",
        "repository_metadata",
        "instrument_categories",
        "instruments",
        "calibration_records",
        "calibration_events",
        "instrument_documents",
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

fn table_exists(connection: &Connection, table: &str) -> Result<bool, AgentError> {
    let count: u32 = connection
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
            params![table],
            |row| row.get(0),
        )
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
        "calibration_due_warning_days, ",
        "created_at, updated_at FROM instruments "
    );
    format!("{base}{suffix}")
}

fn stored_instrument_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<StoredInstrument> {
    let asset_id: String = row.get(0)?;
    let updated_at: String = row.get(18)?;
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
        created_at: row.get(17)?,
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
                "metrology_notes, serviceability_status, ",
                "serviceability_reason, serviceability_updated_at, legacy_availability, created_at, updated_at) ",
                "VALUES (?1, ?2, ?3, ?4, ?5, 'available', ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, 'available', ?15, ?15)"
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
