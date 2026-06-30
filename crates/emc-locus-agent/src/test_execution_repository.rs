use crate::{project_repository::table_exists, AgentError};
use rusqlite::{params, Connection, OptionalExtension, Transaction};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct StoredSimulatedTestExecution {
    pub(crate) attempt_id: String,
    pub(crate) project_code: String,
    pub(crate) test_type: String,
    pub(crate) test_method_reference: String,
    pub(crate) execution_mode: String,
    pub(crate) operator: String,
    pub(crate) checked_on: String,
    pub(crate) status: String,
    pub(crate) readiness_ready: bool,
    pub(crate) readiness_report_json: String,
    pub(crate) refusal_json: Option<String>,
    pub(crate) instrumentation_snapshot_json: String,
    pub(crate) simulation_result_json: Option<String>,
    pub(crate) software_version: String,
    pub(crate) started_at: String,
    pub(crate) completed_at: String,
    pub(crate) revision: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct StoredExecutionInstrument {
    pub(crate) attempt_id: String,
    pub(crate) asset_id: String,
    pub(crate) role: String,
    pub(crate) serviceability_status: Option<String>,
    pub(crate) calibration_requirement: Option<String>,
    pub(crate) calibration_status: String,
    pub(crate) due_at: Option<String>,
    pub(crate) blocking: bool,
    pub(crate) reasons_json: String,
    pub(crate) instrument_revision: Option<String>,
    pub(crate) calibration_revision: Option<String>,
}

pub(crate) struct InsertSimulatedExecutionInput<'a> {
    pub(crate) attempt_id: &'a str,
    pub(crate) project_code: &'a str,
    pub(crate) test_type: &'a str,
    pub(crate) test_method_reference: &'a str,
    pub(crate) execution_mode: &'a str,
    pub(crate) operator: &'a str,
    pub(crate) checked_on: &'a str,
    pub(crate) status: &'a str,
    pub(crate) readiness_ready: bool,
    pub(crate) readiness_report_json: &'a str,
    pub(crate) refusal_json: Option<&'a str>,
    pub(crate) instrumentation_snapshot_json: &'a str,
    pub(crate) simulation_result_json: Option<&'a str>,
    pub(crate) software_version: &'a str,
    pub(crate) started_at: &'a str,
    pub(crate) completed_at: &'a str,
    pub(crate) revision: &'a str,
}

pub(crate) struct InsertExecutionInstrumentInput<'a> {
    pub(crate) attempt_id: &'a str,
    pub(crate) asset_id: &'a str,
    pub(crate) role: &'a str,
    pub(crate) serviceability_status: Option<&'a str>,
    pub(crate) calibration_requirement: Option<&'a str>,
    pub(crate) calibration_status: &'a str,
    pub(crate) due_at: Option<&'a str>,
    pub(crate) blocking: bool,
    pub(crate) reasons_json: &'a str,
    pub(crate) instrument_revision: Option<&'a str>,
    pub(crate) calibration_revision: Option<&'a str>,
}

pub(crate) fn ensure_simulated_execution_tables(connection: &Connection) -> Result<(), AgentError> {
    for table in [
        "simulated_test_executions",
        "simulated_test_execution_instruments",
    ] {
        if !table_exists(connection, "main", table)? {
            return Err(AgentError::new(
                "storage_not_initialized",
                format!("missing required table main.{table}"),
            ));
        }
    }
    Ok(())
}

pub(crate) fn insert_simulated_execution(
    transaction: &Transaction<'_>,
    input: InsertSimulatedExecutionInput<'_>,
) -> Result<(), AgentError> {
    transaction
        .execute(
            concat!(
                "INSERT INTO simulated_test_executions ",
                "(attempt_id, project_code, test_type, test_method_reference, execution_mode, ",
                "operator, checked_on, status, readiness_ready, readiness_report_json, refusal_json, ",
                "instrumentation_snapshot_json, simulation_result_json, software_version, ",
                "started_at, completed_at, revision) ",
                "VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)"
            ),
            params![
                input.attempt_id,
                input.project_code,
                input.test_type,
                input.test_method_reference,
                input.execution_mode,
                input.operator,
                input.checked_on,
                input.status,
                if input.readiness_ready { 1 } else { 0 },
                input.readiness_report_json,
                input.refusal_json,
                input.instrumentation_snapshot_json,
                input.simulation_result_json,
                input.software_version,
                input.started_at,
                input.completed_at,
                input.revision,
            ],
        )
        .map_err(|error| AgentError::new("test_execution_write_failed", error.to_string()))?;
    Ok(())
}

pub(crate) fn insert_execution_instrument(
    transaction: &Transaction<'_>,
    input: InsertExecutionInstrumentInput<'_>,
) -> Result<(), AgentError> {
    transaction
        .execute(
            concat!(
                "INSERT INTO simulated_test_execution_instruments ",
                "(attempt_id, asset_id, role, serviceability_status, calibration_requirement, ",
                "calibration_status, due_at, blocking, reasons_json, instrument_revision, calibration_revision) ",
                "VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)"
            ),
            params![
                input.attempt_id,
                input.asset_id,
                input.role,
                input.serviceability_status,
                input.calibration_requirement,
                input.calibration_status,
                input.due_at,
                if input.blocking { 1 } else { 0 },
                input.reasons_json,
                input.instrument_revision,
                input.calibration_revision,
            ],
        )
        .map_err(|error| {
            AgentError::new("test_execution_instrument_write_failed", error.to_string())
        })?;
    Ok(())
}

pub(crate) fn load_simulated_execution(
    connection: &Connection,
    attempt_id: &str,
) -> Result<Option<StoredSimulatedTestExecution>, AgentError> {
    connection
        .query_row(
            concat!(
                "SELECT attempt_id, project_code, test_type, test_method_reference, execution_mode, ",
                "operator, checked_on, status, readiness_ready, readiness_report_json, refusal_json, ",
                "instrumentation_snapshot_json, simulation_result_json, software_version, ",
                "started_at, completed_at, revision ",
                "FROM simulated_test_executions WHERE attempt_id = ?1"
            ),
            params![attempt_id],
            stored_execution_from_row,
        )
        .optional()
        .map_err(|error| AgentError::new("test_execution_query_failed", error.to_string()))
}

pub(crate) fn load_project_simulated_executions(
    connection: &Connection,
    project_code: &str,
) -> Result<Vec<StoredSimulatedTestExecution>, AgentError> {
    let mut statement = connection
        .prepare(concat!(
            "SELECT attempt_id, project_code, test_type, test_method_reference, execution_mode, ",
            "operator, checked_on, status, readiness_ready, readiness_report_json, refusal_json, ",
            "instrumentation_snapshot_json, simulation_result_json, software_version, ",
            "started_at, completed_at, revision ",
            "FROM simulated_test_executions WHERE project_code = ?1 ",
            "ORDER BY completed_at DESC, attempt_id"
        ))
        .map_err(|error| AgentError::new("test_execution_query_failed", error.to_string()))?;
    let rows = statement
        .query_map(params![project_code], stored_execution_from_row)
        .map_err(|error| AgentError::new("test_execution_query_failed", error.to_string()))?;
    let mut executions = Vec::new();
    for row in rows {
        executions.push(
            row.map_err(|error| AgentError::new("test_execution_query_failed", error.to_string()))?,
        );
    }
    Ok(executions)
}

pub(crate) fn load_execution_instruments(
    connection: &Connection,
    attempt_id: &str,
) -> Result<Vec<StoredExecutionInstrument>, AgentError> {
    let mut statement = connection
        .prepare(
            concat!(
                "SELECT attempt_id, asset_id, role, serviceability_status, calibration_requirement, ",
                "calibration_status, due_at, blocking, reasons_json, instrument_revision, calibration_revision ",
                "FROM simulated_test_execution_instruments WHERE attempt_id = ?1 ORDER BY asset_id, role"
            ),
        )
        .map_err(|error| {
            AgentError::new("test_execution_instrument_query_failed", error.to_string())
        })?;
    let rows = statement
        .query_map(params![attempt_id], |row| {
            Ok(StoredExecutionInstrument {
                attempt_id: row.get(0)?,
                asset_id: row.get(1)?,
                role: row.get(2)?,
                serviceability_status: row.get(3)?,
                calibration_requirement: row.get(4)?,
                calibration_status: row.get(5)?,
                due_at: row.get(6)?,
                blocking: row.get::<_, u8>(7)? == 1,
                reasons_json: row.get(8)?,
                instrument_revision: row.get(9)?,
                calibration_revision: row.get(10)?,
            })
        })
        .map_err(|error| {
            AgentError::new("test_execution_instrument_query_failed", error.to_string())
        })?;
    let mut instruments = Vec::new();
    for row in rows {
        instruments.push(row.map_err(|error| {
            AgentError::new("test_execution_instrument_query_failed", error.to_string())
        })?);
    }
    Ok(instruments)
}

fn stored_execution_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<StoredSimulatedTestExecution> {
    Ok(StoredSimulatedTestExecution {
        attempt_id: row.get(0)?,
        project_code: row.get(1)?,
        test_type: row.get(2)?,
        test_method_reference: row.get(3)?,
        execution_mode: row.get(4)?,
        operator: row.get(5)?,
        checked_on: row.get(6)?,
        status: row.get(7)?,
        readiness_ready: row.get::<_, u8>(8)? == 1,
        readiness_report_json: row.get(9)?,
        refusal_json: row.get(10)?,
        instrumentation_snapshot_json: row.get(11)?,
        simulation_result_json: row.get(12)?,
        software_version: row.get(13)?,
        started_at: row.get(14)?,
        completed_at: row.get(15)?,
        revision: row.get(16)?,
    })
}
