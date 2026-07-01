use crate::metrology_dto::{ReadinessInstrumentResultDto, ReadinessIssueDto, ReadinessReportDto};
use crate::metrology_service::{assess_metrology_readiness_report, AssessReadinessInput};
use crate::project_repository::{
    ensure_operation_replay, existing_operation, insert_audit_event, insert_sync_operation,
    load_project, next_audit_sequence, open_project_connection, AuditEventInput,
    OperationFingerprintInput, SyncOperationInput,
};
use crate::test_execution_dto::{
    ExecutionInstrumentSnapshotDto, ExecutionRefusalCauseDto, ExecutionRefusalDto,
    SimulatedEmcResultDto, SimulatedTestExecutionDto, SimulatedTestExecutionEnvelopeDto,
    SimulatedTestExecutionListDto, SimulatedTestExecutionResultDto,
    SimulatedTestExecutionSummaryDto,
};
use crate::test_execution_repository::{
    ensure_simulated_execution_tables, insert_execution_instrument, insert_simulated_execution,
    load_execution_instruments, load_project_simulated_executions, load_simulated_execution,
    InsertExecutionInstrumentInput, InsertSimulatedExecutionInput, StoredSimulatedTestExecution,
};
use crate::test_template_repository::{load_test_template_identity, open_test_template_connection};
use crate::{render_json, AgentError};
use emc_locus_core::{
    AuditActor, AuditReason, ExecutionMode, InstrumentCode, MeasurementRunReference, MetrologyDate,
    ProjectCode, StableId, TestMethodReference,
};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::path::Path;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RunSimulatedEmcTestInput {
    pub(crate) attempt_id: String,
    pub(crate) project_code: String,
    pub(crate) test_method_reference: String,
    pub(crate) execution_mode: String,
    pub(crate) required_asset_ids: Vec<String>,
    pub(crate) operator: String,
    pub(crate) checked_on: String,
    pub(crate) reason: String,
    pub(crate) operation_id: String,
    pub(crate) correlation_id: String,
    pub(crate) device_id: String,
}

pub(crate) fn run_simulated_emc_test(
    storage_root: &Path,
    input: RunSimulatedEmcTestInput,
) -> Result<String, AgentError> {
    validate_simulated_emc_input(&input)?;

    let command_payload = command_payload_json(&input);
    let mut connection = open_project_connection(storage_root)?;
    ensure_simulated_execution_tables(&connection)?;

    if let Some(operation) = existing_operation(&connection, &input.operation_id)? {
        ensure_operation_replay(
            &operation,
            &input.operation_id,
            OperationFingerprintInput {
                domain: "project_records",
                entity_type: "simulated_test_execution",
                entity_id: &input.attempt_id,
                operation_kind: &operation.operation_kind,
                base_revision: &operation.base_revision,
                actor_id: &input.operator,
                device_id: &input.device_id,
                correlation_id: &input.correlation_id,
                payload_json: &command_payload,
            },
        )?;
        let execution = load_execution_dto(&connection, &input.attempt_id)?.ok_or_else(|| {
            AgentError::new(
                "operation_replay_missing_entity",
                "operation exists but simulated test execution is missing",
            )
        })?;
        return Ok(render_json(&SimulatedTestExecutionResultDto {
            operation: operation.operation_kind,
            operation_id: operation.operation_id,
            replayed: true,
            execution,
        }));
    }

    load_project(&connection, &input.project_code)?.ok_or_else(|| {
        AgentError::new(
            "project_not_found",
            "simulated test execution requires an existing project",
        )
    })?;
    ensure_approved_template_reference(storage_root, &input.test_method_reference)?;
    if load_simulated_execution(&connection, &input.attempt_id)?.is_some() {
        return Err(AgentError::new(
            "test_execution_attempt_exists",
            format!(
                "test execution attempt already exists: {}",
                input.attempt_id
            ),
        ));
    }

    let readiness = assess_metrology_readiness_report(
        storage_root,
        AssessReadinessInput {
            asset_ids: input.required_asset_ids.clone(),
            execution_mode: input.execution_mode.clone(),
            checked_on: input.checked_on.clone(),
            context: Some(readiness_context(&input)),
        },
    )?;
    let instrumentation_snapshot = instrumentation_snapshot(&readiness);
    let refusal = if readiness.ready {
        None
    } else {
        Some(refusal_from_readiness(&readiness))
    };
    let simulation_result = if readiness.ready {
        Some(simulated_emc_result(&input))
    } else {
        None
    };
    let status = if readiness.ready {
        "completed"
    } else {
        "refused"
    };
    let operation_kind = if readiness.ready {
        "simulated_test_execution_completed"
    } else {
        "simulated_test_execution_refused"
    };
    let now = utc_timestamp()?;
    let revision = revision_for("simulated_test_execution", &input.attempt_id, &now);
    let readiness_json = render_json(&readiness);
    let refusal_json = refusal.as_ref().map(render_json);
    let snapshot_json = render_json(&instrumentation_snapshot);
    let result_json = simulation_result.as_ref().map(render_json);
    let audit_payload = audit_payload_json(
        &input,
        status,
        &readiness,
        refusal.as_ref(),
        simulation_result.as_ref(),
    );

    let sequence = next_audit_sequence(&connection, &input.project_code)?;
    let transaction = connection
        .transaction()
        .map_err(|error| AgentError::new("transaction_begin_failed", error.to_string()))?;
    insert_simulated_execution(
        &transaction,
        InsertSimulatedExecutionInput {
            attempt_id: &input.attempt_id,
            project_code: &input.project_code,
            test_type: "simulated_emc",
            test_method_reference: &input.test_method_reference,
            execution_mode: &input.execution_mode,
            operator: &input.operator,
            checked_on: &input.checked_on,
            status,
            readiness_ready: readiness.ready,
            readiness_report_json: &readiness_json,
            refusal_json: refusal_json.as_deref(),
            instrumentation_snapshot_json: &snapshot_json,
            simulation_result_json: result_json.as_deref(),
            software_version: env!("CARGO_PKG_VERSION"),
            started_at: &now,
            completed_at: &now,
            revision: &revision,
        },
    )?;
    for snapshot in &instrumentation_snapshot {
        let reasons_json = render_json(&snapshot.reasons);
        insert_execution_instrument(
            &transaction,
            InsertExecutionInstrumentInput {
                attempt_id: &input.attempt_id,
                asset_id: &snapshot.asset_id,
                role: &snapshot.role,
                serviceability_status: snapshot.serviceability_status.as_deref(),
                calibration_requirement: snapshot.calibration_requirement.as_deref(),
                calibration_status: &snapshot.calibration_status,
                due_at: snapshot.due_at.as_deref(),
                blocking: snapshot.blocking,
                reasons_json: &reasons_json,
                instrument_revision: snapshot.instrument_revision.as_deref(),
                calibration_revision: snapshot.calibration_revision.as_deref(),
            },
        )?;
    }
    insert_audit_event(
        &transaction,
        AuditEventInput {
            project_code: &input.project_code,
            sequence,
            actor: &input.operator,
            action: operation_kind,
            reason: Some(&input.reason),
            payload_json: &audit_payload,
            timestamp: &now,
        },
    )?;
    insert_sync_operation(
        &transaction,
        SyncOperationInput {
            domain: "project_records",
            entity_type: "simulated_test_execution",
            operation_id: &input.operation_id,
            entity_id: &input.attempt_id,
            operation_kind,
            base_revision: "rev-0000",
            resulting_revision: &revision,
            actor_id: &input.operator,
            device_id: &input.device_id,
            correlation_id: &input.correlation_id,
            payload_json: &command_payload,
            timestamp: &now,
        },
    )?;
    transaction
        .commit()
        .map_err(|error| AgentError::new("transaction_commit_failed", error.to_string()))?;

    let execution = load_execution_dto(&connection, &input.attempt_id)?.ok_or_else(|| {
        AgentError::new(
            "test_execution_read_failed",
            "created simulated test execution could not be reloaded",
        )
    })?;
    Ok(render_json(&SimulatedTestExecutionResultDto {
        operation: operation_kind.to_owned(),
        operation_id: input.operation_id,
        replayed: false,
        execution,
    }))
}

pub(crate) fn get_simulated_test_execution(
    storage_root: &Path,
    attempt_id: &str,
) -> Result<String, AgentError> {
    StableId::parse(attempt_id.to_owned()).map_err(domain_error)?;
    let connection = open_project_connection(storage_root)?;
    ensure_simulated_execution_tables(&connection)?;
    let execution = load_execution_dto(&connection, attempt_id)?
        .ok_or_else(|| AgentError::new("test_execution_not_found", "test execution not found"))?;
    Ok(render_json(&SimulatedTestExecutionEnvelopeDto {
        execution,
    }))
}

pub(crate) fn list_project_simulated_test_executions(
    storage_root: &Path,
    project_code: &str,
) -> Result<String, AgentError> {
    let project_code = ProjectCode::parse(project_code.to_owned()).map_err(domain_error)?;
    let connection = open_project_connection(storage_root)?;
    ensure_simulated_execution_tables(&connection)?;
    load_project(&connection, project_code.as_str())?
        .ok_or_else(|| AgentError::new("project_not_found", "project does not exist"))?;
    let executions = load_project_simulated_executions(&connection, project_code.as_str())?;
    Ok(render_json(&SimulatedTestExecutionListDto {
        executions: executions.iter().map(execution_summary_dto).collect(),
    }))
}

fn validate_simulated_emc_input(input: &RunSimulatedEmcTestInput) -> Result<(), AgentError> {
    MeasurementRunReference::parse(input.attempt_id.clone()).map_err(domain_error)?;
    ProjectCode::parse(input.project_code.clone()).map_err(domain_error)?;
    TestMethodReference::parse(input.test_method_reference.clone()).map_err(domain_error)?;
    parse_execution_mode(&input.execution_mode)?;
    parse_metrology_date(&input.checked_on, "checked_on")?;
    AuditActor::parse(input.operator.clone()).map_err(domain_error)?;
    AuditReason::parse(input.reason.clone()).map_err(domain_error)?;
    StableId::parse(input.operation_id.clone()).map_err(domain_error)?;
    StableId::parse(input.correlation_id.clone()).map_err(domain_error)?;
    StableId::parse(input.device_id.clone()).map_err(domain_error)?;
    if input.required_asset_ids.is_empty() {
        return Err(AgentError::new(
            "invalid_test_execution",
            "required_asset_ids must not be empty",
        ));
    }
    for asset_id in &input.required_asset_ids {
        InstrumentCode::parse(asset_id.clone()).map_err(domain_error)?;
    }
    Ok(())
}

fn ensure_approved_template_reference(
    storage_root: &Path,
    test_method_reference: &str,
) -> Result<(), AgentError> {
    let connection = match open_test_template_connection(storage_root) {
        Ok(connection) => connection,
        Err(error) if error.code == "storage_not_initialized" => return Ok(()),
        Err(error) => return Err(error),
    };
    let Some(template) = load_test_template_identity(&connection, test_method_reference)? else {
        return Ok(());
    };
    if template.current_approved_revision_id.is_some() {
        return Ok(());
    }
    Err(AgentError::with_details(
        "test_execution_template_not_approved",
        "simulated test execution requires a current approved test-template revision",
        json!({
            "template_id": template.template_id,
            "current_approved_revision_id": template.current_approved_revision_id,
        }),
    ))
}

fn instrumentation_snapshot(readiness: &ReadinessReportDto) -> Vec<ExecutionInstrumentSnapshotDto> {
    readiness
        .instrument_results
        .iter()
        .enumerate()
        .map(|(index, result)| snapshot_from_readiness_result(index, result))
        .collect()
}

fn snapshot_from_readiness_result(
    index: usize,
    result: &ReadinessInstrumentResultDto,
) -> ExecutionInstrumentSnapshotDto {
    ExecutionInstrumentSnapshotDto {
        asset_id: result.asset_id.clone(),
        role: if index == 0 { "primary" } else { "support" }.to_owned(),
        serviceability_status: result.serviceability_status.clone(),
        calibration_requirement: result.calibration_requirement.clone(),
        calibration_status: result.calibration_status.clone(),
        due_at: result.due_at.clone(),
        blocking: result.blocking,
        reasons: result.reasons.clone(),
        instrument_revision: result.instrument_revision.clone(),
        calibration_revision: result.calibration_revision.clone(),
    }
}

fn refusal_from_readiness(readiness: &ReadinessReportDto) -> ExecutionRefusalDto {
    ExecutionRefusalDto {
        code: "equipment_readiness_blocked".to_owned(),
        message: "Execution refused because required instrumentation is not ready".to_owned(),
        causes: readiness
            .blocking_issues
            .iter()
            .map(refusal_cause_from_issue)
            .collect(),
    }
}

fn refusal_cause_from_issue(issue: &ReadinessIssueDto) -> ExecutionRefusalCauseDto {
    ExecutionRefusalCauseDto {
        code: issue.code.clone(),
        message: issue.message.clone(),
        asset_id: issue.asset_id.clone(),
        dimension: issue.dimension.clone(),
    }
}

fn simulated_emc_result(input: &RunSimulatedEmcTestInput) -> SimulatedEmcResultDto {
    let seed = stable_seed(&format!(
        "{}:{}:{}",
        input.attempt_id, input.project_code, input.test_method_reference
    ));
    let span = 30_000_000_u64 - 150_000_u64;
    let peak_frequency_hz = 150_000 + seed % span;
    let peak_level_dbuv = rounded_tenth(42.0 + (seed % 180) as f64 / 10.0);
    let limit_dbuv = 66.0;
    let margin_db = rounded_tenth(limit_dbuv - peak_level_dbuv);
    SimulatedEmcResultDto {
        strategy: "deterministic_conducted_emission_level_sweep".to_owned(),
        verdict: if margin_db >= 0.0 { "pass" } else { "fail" }.to_owned(),
        measurement_axis: "frequency".to_owned(),
        start_frequency_hz: 150_000,
        stop_frequency_hz: 30_000_000,
        points: 401,
        peak_frequency_hz,
        peak_level_dbuv,
        limit_dbuv,
        margin_db,
    }
}

fn load_execution_dto(
    connection: &rusqlite::Connection,
    attempt_id: &str,
) -> Result<Option<SimulatedTestExecutionDto>, AgentError> {
    let Some(execution) = load_simulated_execution(connection, attempt_id)? else {
        return Ok(None);
    };
    let instruments = load_execution_instruments(connection, attempt_id)?;
    let snapshot = instruments
        .iter()
        .map(|instrument| {
            let reasons =
                serde_json::from_str::<Vec<String>>(&instrument.reasons_json).map_err(|error| {
                    AgentError::new("test_execution_decode_failed", error.to_string())
                })?;
            Ok(ExecutionInstrumentSnapshotDto {
                asset_id: instrument.asset_id.clone(),
                role: instrument.role.clone(),
                serviceability_status: instrument.serviceability_status.clone(),
                calibration_requirement: instrument.calibration_requirement.clone(),
                calibration_status: instrument.calibration_status.clone(),
                due_at: instrument.due_at.clone(),
                blocking: instrument.blocking,
                reasons,
                instrument_revision: instrument.instrument_revision.clone(),
                calibration_revision: instrument.calibration_revision.clone(),
            })
        })
        .collect::<Result<Vec<_>, AgentError>>()?;
    Ok(Some(execution_dto(&execution, snapshot)?))
}

fn execution_dto(
    execution: &StoredSimulatedTestExecution,
    instrumentation_snapshot: Vec<ExecutionInstrumentSnapshotDto>,
) -> Result<SimulatedTestExecutionDto, AgentError> {
    let readiness = serde_json::from_str::<ReadinessReportDto>(&execution.readiness_report_json)
        .map_err(|error| AgentError::new("test_execution_decode_failed", error.to_string()))?;
    let refusal = execution
        .refusal_json
        .as_deref()
        .map(serde_json::from_str::<ExecutionRefusalDto>)
        .transpose()
        .map_err(|error| AgentError::new("test_execution_decode_failed", error.to_string()))?;
    let simulation_result = execution
        .simulation_result_json
        .as_deref()
        .map(serde_json::from_str::<SimulatedEmcResultDto>)
        .transpose()
        .map_err(|error| AgentError::new("test_execution_decode_failed", error.to_string()))?;
    Ok(SimulatedTestExecutionDto {
        attempt_id: execution.attempt_id.clone(),
        project_code: execution.project_code.clone(),
        test_type: execution.test_type.clone(),
        test_method_reference: execution.test_method_reference.clone(),
        execution_mode: execution.execution_mode.clone(),
        operator: execution.operator.clone(),
        checked_on: execution.checked_on.clone(),
        status: execution.status.clone(),
        readiness,
        refusal,
        instrumentation_snapshot,
        simulation_result,
        software_version: execution.software_version.clone(),
        started_at: execution.started_at.clone(),
        completed_at: execution.completed_at.clone(),
        revision: execution.revision.clone(),
    })
}

fn execution_summary_dto(
    execution: &StoredSimulatedTestExecution,
) -> SimulatedTestExecutionSummaryDto {
    SimulatedTestExecutionSummaryDto {
        attempt_id: execution.attempt_id.clone(),
        project_code: execution.project_code.clone(),
        test_method_reference: execution.test_method_reference.clone(),
        execution_mode: execution.execution_mode.clone(),
        operator: execution.operator.clone(),
        status: execution.status.clone(),
        ready: execution.readiness_ready,
        checked_on: execution.checked_on.clone(),
        completed_at: execution.completed_at.clone(),
        revision: execution.revision.clone(),
    }
}

fn readiness_context(input: &RunSimulatedEmcTestInput) -> String {
    format!(
        "simulated_emc:{}:{}:{}",
        input.project_code, input.attempt_id, input.test_method_reference
    )
}

fn command_payload_json(input: &RunSimulatedEmcTestInput) -> String {
    render_json(&json!({
        "attempt_id": input.attempt_id,
        "project_code": input.project_code,
        "test_type": "simulated_emc",
        "test_method_reference": input.test_method_reference,
        "execution_mode": input.execution_mode,
        "required_asset_ids": input.required_asset_ids,
        "operator": input.operator,
        "checked_on": input.checked_on,
        "reason": input.reason,
    }))
}

fn audit_payload_json(
    input: &RunSimulatedEmcTestInput,
    status: &str,
    readiness: &ReadinessReportDto,
    refusal: Option<&ExecutionRefusalDto>,
    simulation_result: Option<&SimulatedEmcResultDto>,
) -> String {
    render_json(&json!({
        "command": serde_json::from_str::<serde_json::Value>(&command_payload_json(input))
            .expect("command payload is valid JSON"),
        "status": status,
        "readiness_ready": readiness.ready,
        "blocking_issue_count": readiness.blocking_issues.len(),
        "warning_count": readiness.warnings.len(),
        "refusal": refusal,
        "simulation_result": simulation_result,
    }))
}

fn parse_execution_mode(mode: &str) -> Result<ExecutionMode, AgentError> {
    match mode {
        "accredited" => Ok(ExecutionMode::Accredited),
        "non_accredited" => Ok(ExecutionMode::NonAccredited),
        "investigation" => Ok(ExecutionMode::Investigation),
        other => Err(AgentError::new(
            "unknown_execution_mode",
            format!("unknown execution mode: {other}"),
        )),
    }
}

fn parse_metrology_date(value: &str, field: &'static str) -> Result<MetrologyDate, AgentError> {
    let parts = value.trim().split('-').collect::<Vec<_>>();
    if parts.len() != 3 || parts[0].len() != 4 || parts[1].len() != 2 || parts[2].len() != 2 {
        return Err(AgentError::new(
            "invalid_metrology_date",
            format!("{field} must use YYYY-MM-DD"),
        ));
    }
    let year = parts[0].parse::<u16>().map_err(|_| {
        AgentError::new(
            "invalid_metrology_date",
            format!("{field} must use YYYY-MM-DD"),
        )
    })?;
    let month = parts[1].parse::<u8>().map_err(|_| {
        AgentError::new(
            "invalid_metrology_date",
            format!("{field} must use YYYY-MM-DD"),
        )
    })?;
    let day = parts[2].parse::<u8>().map_err(|_| {
        AgentError::new(
            "invalid_metrology_date",
            format!("{field} must use YYYY-MM-DD"),
        )
    })?;
    MetrologyDate::new(year, month, day).map_err(domain_error)
}

fn domain_error(error: emc_locus_core::DomainError) -> AgentError {
    AgentError::new("domain_error", format!("{error:?}"))
}

fn utc_timestamp() -> Result<String, AgentError> {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .map_err(|error| AgentError::new("timestamp_format_failed", error.to_string()))
}

fn revision_for(entity_type: &str, entity_id: &str, updated_at: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"emc-locus-agent:");
    hasher.update(entity_type.as_bytes());
    hasher.update(b":");
    hasher.update(entity_id.as_bytes());
    hasher.update(b":");
    hasher.update(updated_at.as_bytes());
    let digest = format!("{:x}", hasher.finalize());
    format!("rev-{}", &digest[..12])
}

fn stable_seed(value: &str) -> u64 {
    let digest = Sha256::digest(value.as_bytes());
    let mut bytes = [0_u8; 8];
    bytes.copy_from_slice(&digest[..8]);
    u64::from_be_bytes(bytes)
}

fn rounded_tenth(value: f64) -> f64 {
    (value * 10.0).round() / 10.0
}
