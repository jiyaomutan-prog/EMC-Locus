use crate::{
    project_repository::{
        insert_audit_event, insert_sync_operation, next_audit_sequence,
        open_start_consistency_connection, AuditEventInput, SyncOperationInput,
    },
    render_json, AgentError,
};
use emc_locus_core::{
    compile_execution_plan, ExecutionConfigurationDefinition, MeasurementSystemTemplateDefinition,
    PlannedTestPreparationDefinition, RegulationProfileDefinition, TestMethodDefinitionV2,
    VersionedMethodDefinition,
};
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, path::Path};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExecutionConfigurationOperationContext {
    pub actor: String,
    pub reason: String,
    pub operation_id: String,
    pub correlation_id: String,
    pub device_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeriveExecutionConfigurationInput {
    pub project_code: String,
    pub schedule_item_code: String,
    pub planned_preparation_revision_id: String,
    pub expected_current_revision_id: Option<String>,
    pub definition_json: String,
    pub regulation_profiles_json: Vec<String>,
    pub context: ExecutionConfigurationOperationContext,
}

pub fn derive_execution_configuration(
    storage_root: &Path,
    input: DeriveExecutionConfigurationInput,
) -> Result<String, AgentError> {
    validate_input(&input)?;
    let mut connection = open_start_consistency_connection(storage_root)?;
    let definition: ExecutionConfigurationDefinition = serde_json::from_str(&input.definition_json)
        .map_err(|error| {
            AgentError::new("invalid_execution_configuration_json", error.to_string())
        })?;
    let method = load_method(&connection, &definition)?;
    let system = load_system(&connection, &definition)?;
    let profiles = input
        .regulation_profiles_json
        .iter()
        .map(|value| {
            serde_json::from_str::<RegulationProfileDefinition>(value).map_err(|error| {
                AgentError::new("invalid_regulation_profile_json", error.to_string())
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let canonical = definition
        .canonicalize(&method, &system)
        .map_err(validation_error)?;
    verify_station_pin(&connection, &definition)?;
    let preparation = load_preparation(&connection, &input, &definition)?;
    verify_assignment_source(&definition, &preparation)?;
    let preview = compile_execution_plan(&method, &system, &profiles, Some(&definition));
    let readiness_json = render_json(&json!({
        "ready": preview.blockers.is_empty(),
        "blockers": preview.blockers,
        "warnings": preview.warnings,
        "unresolved_roles": preview.unresolved_roles,
    }));
    let request_payload = render_json(&json!({
        "project_code": input.project_code,
        "schedule_item_code": input.schedule_item_code,
        "planned_preparation_revision_id": input.planned_preparation_revision_id,
        "expected_current_revision_id": input.expected_current_revision_id,
        "definition_checksum": canonical.definition_checksum,
        "reason": input.context.reason,
    }));
    let request_checksum = operation_checksum(&request_payload, &input.context);
    if let Some(replay) = load_replay(&connection, &input.context.operation_id, &request_checksum)?
    {
        return Ok(replay);
    }

    let current = connection
        .query_row(
            "SELECT current_revision_id FROM execution_configuration_identities WHERE configuration_id = ?1",
            params![definition.configuration_id],
            |row| row.get::<_, Option<String>>(0),
        )
        .optional()
        .map_err(sql_error("execution_configuration_query_failed"))?
        .flatten();
    if current != input.expected_current_revision_id {
        return Err(AgentError::with_details(
            "execution_configuration_revision_conflict",
            "the dated execution configuration changed since it was loaded",
            json!({
                "expected_current_revision_id": input.expected_current_revision_id,
                "actual_current_revision_id": current,
            }),
        ));
    }
    let revision_number: u32 = connection
        .query_row(
            "SELECT COALESCE(MAX(revision_number), 0) + 1 FROM execution_configuration_revisions WHERE configuration_id = ?1",
            params![definition.configuration_id],
            |row| row.get(0),
        )
        .map_err(sql_error("execution_configuration_query_failed"))?;
    let revision_id = format!("{}-rev-{revision_number:04}", definition.configuration_id);
    let now = utc_timestamp()?;
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(sql_error("execution_configuration_transaction_failed"))?;
    transaction
        .execute(
            "INSERT OR IGNORE INTO execution_configuration_identities (configuration_id, project_code, schedule_item_code, current_revision_id, created_by, created_at, updated_at) VALUES (?1, ?2, ?3, NULL, ?4, ?5, ?5)",
            params![definition.configuration_id, input.project_code, input.schedule_item_code, input.context.actor, now],
        )
        .map_err(sql_error("execution_configuration_write_failed"))?;
    transaction
        .execute(
            "INSERT INTO execution_configuration_revisions (revision_id, configuration_id, revision_number, parent_revision_id, method_template_id, method_revision_id, method_definition_checksum, system_template_id, system_template_revision_id, system_template_definition_checksum, station_setup_id, station_setup_revision_id, station_setup_definition_checksum, planned_preparation_revision_id, definition_schema_version, definition_json, definition_checksum, readiness_json, operation_id, request_checksum, actor, reason, device_id, correlation_id, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25)",
            params![
                revision_id,
                definition.configuration_id,
                revision_number,
                current,
                definition.method_revision.identity_id,
                definition.method_revision.revision_id,
                definition.method_revision.definition_checksum,
                definition.measurement_system_template_revision.identity_id,
                definition.measurement_system_template_revision.revision_id,
                definition.measurement_system_template_revision.definition_checksum,
                definition.station_setup_revision.identity_id,
                definition.station_setup_revision.revision_id,
                definition.station_setup_revision.definition_checksum,
                input.planned_preparation_revision_id,
                canonical.definition_schema_version,
                canonical.canonical_json,
                canonical.definition_checksum,
                readiness_json,
                input.context.operation_id,
                request_checksum,
                input.context.actor,
                input.context.reason,
                input.context.device_id,
                input.context.correlation_id,
                now,
            ],
        )
        .map_err(sql_error("execution_configuration_write_failed"))?;
    transaction
        .execute(
            "UPDATE execution_configuration_identities SET current_revision_id = ?2, updated_at = ?3 WHERE configuration_id = ?1",
            params![definition.configuration_id, revision_id, now],
        )
        .map_err(sql_error("execution_configuration_write_failed"))?;
    let audit_sequence = next_audit_sequence(&transaction, &input.project_code)?;
    insert_audit_event(
        &transaction,
        AuditEventInput {
            project_code: &input.project_code,
            sequence: audit_sequence,
            actor: &input.context.actor,
            action: "execution_configuration_derived",
            reason: Some(&input.context.reason),
            payload_json: &request_payload,
            timestamp: &now,
        },
    )?;
    insert_sync_operation(
        &transaction,
        SyncOperationInput {
            domain: "project_records",
            entity_type: "execution_configuration",
            operation_id: &input.context.operation_id,
            entity_id: &definition.configuration_id,
            operation_kind: "execution_configuration_derived",
            base_revision: current.as_deref().unwrap_or("none"),
            resulting_revision: &revision_id,
            actor_id: &input.context.actor,
            device_id: &input.context.device_id,
            correlation_id: &input.context.correlation_id,
            payload_json: &request_payload,
            timestamp: &now,
        },
    )?;
    transaction
        .commit()
        .map_err(sql_error("execution_configuration_commit_failed"))?;
    execution_configuration_result(&connection, &definition.configuration_id, false)
}

pub fn get_execution_configuration(
    storage_root: &Path,
    configuration_id: &str,
) -> Result<String, AgentError> {
    let connection = open_start_consistency_connection(storage_root)?;
    execution_configuration_result(&connection, configuration_id, false)
}

fn load_method(
    connection: &Connection,
    definition: &ExecutionConfigurationDefinition,
) -> Result<TestMethodDefinitionV2, AgentError> {
    let stored: Option<(String, String, String)> = connection
        .query_row(
            "SELECT status, definition_json, definition_checksum FROM test_definitions_db.test_template_revisions WHERE template_id = ?1 AND revision_id = ?2",
            params![definition.method_revision.identity_id, definition.method_revision.revision_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()
        .map_err(sql_error("execution_method_query_failed"))?;
    let (status, json, checksum) = stored.ok_or_else(|| {
        AgentError::new(
            "execution_method_revision_not_found",
            "the pinned method revision does not exist",
        )
    })?;
    if status != "approved" || checksum != definition.method_revision.definition_checksum {
        return Err(AgentError::new(
            "execution_method_revision_not_approved",
            "the exact pinned method revision must be approved and checksum-matched",
        ));
    }
    match VersionedMethodDefinition::from_json_str(&json).map_err(validation_error)? {
        VersionedMethodDefinition::V2(method) => Ok(method),
        VersionedMethodDefinition::V1(_) => Err(AgentError::new(
            "execution_method_v2_required",
            "derive and approve a 0.22.2 method successor before dated execution preparation",
        )),
    }
}

fn load_system(
    connection: &Connection,
    definition: &ExecutionConfigurationDefinition,
) -> Result<MeasurementSystemTemplateDefinition, AgentError> {
    let stored: Option<(String, String, String)> = connection
        .query_row(
            "SELECT status, definition_json, definition_checksum FROM test_definitions_db.method_workflow_revisions WHERE aggregate_kind = 'measurement_system_template' AND entity_id = ?1 AND revision_id = ?2",
            params![definition.measurement_system_template_revision.identity_id, definition.measurement_system_template_revision.revision_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()
        .map_err(sql_error("execution_system_query_failed"))?;
    let (status, json, checksum) = stored.ok_or_else(|| {
        AgentError::new(
            "execution_system_revision_not_found",
            "the pinned system-template revision does not exist",
        )
    })?;
    if !matches!(status.as_str(), "validated" | "approved")
        || checksum
            != definition
                .measurement_system_template_revision
                .definition_checksum
    {
        return Err(AgentError::new(
            "execution_system_revision_not_usable",
            "the exact system-template revision must be validated or approved and checksum-matched",
        ));
    }
    serde_json::from_str(&json).map_err(|error| {
        AgentError::new(
            "invalid_measurement_system_template_json",
            error.to_string(),
        )
    })
}

fn verify_station_pin(
    connection: &Connection,
    definition: &ExecutionConfigurationDefinition,
) -> Result<(), AgentError> {
    let stored: Option<(String, String, Option<String>)> = connection
        .query_row(
            "SELECT status, definition_checksum, qualified_at FROM station_db.station_setup_revisions WHERE setup_id = ?1 AND revision_id = ?2",
            params![definition.station_setup_revision.identity_id, definition.station_setup_revision.revision_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()
        .map_err(sql_error("execution_station_query_failed"))?;
    let (status, checksum, qualified_at) = stored.ok_or_else(|| {
        AgentError::new(
            "execution_station_revision_not_found",
            "the pinned station revision does not exist",
        )
    })?;
    if checksum != definition.station_setup_revision.definition_checksum
        || (!matches!(status.as_str(), "ready" | "superseded") && qualified_at.is_none())
    {
        return Err(AgentError::new(
            "execution_station_revision_not_usable",
            "the exact station revision must be qualified or ready and checksum-matched",
        ));
    }
    Ok(())
}

fn load_preparation(
    connection: &Connection,
    input: &DeriveExecutionConfigurationInput,
    definition: &ExecutionConfigurationDefinition,
) -> Result<PlannedTestPreparationDefinition, AgentError> {
    let stored: Option<(String, String, String, String, String, String)> = connection
        .query_row(
            "SELECT method_template_id, method_revision_id, method_definition_checksum, station_setup_revision_id, station_setup_definition_checksum, definition_json FROM planned_test_preparation_revisions WHERE project_code = ?1 AND schedule_item_code = ?2 AND revision_id = ?3",
            params![input.project_code, input.schedule_item_code, input.planned_preparation_revision_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?)),
        )
        .optional()
        .map_err(sql_error("execution_preparation_query_failed"))?;
    let (method_id, method_revision, method_checksum, station_revision, station_checksum, json) =
        stored.ok_or_else(|| {
            AgentError::new(
                "execution_preparation_not_found",
                "the pinned planned-test preparation revision does not exist",
            )
        })?;
    if method_id != definition.method_revision.identity_id
        || method_revision != definition.method_revision.revision_id
        || method_checksum != definition.method_revision.definition_checksum
        || station_revision != definition.station_setup_revision.revision_id
        || station_checksum != definition.station_setup_revision.definition_checksum
    {
        return Err(AgentError::new(
            "execution_preparation_pin_mismatch",
            "method or station pins differ from the authoritative planned-test preparation",
        ));
    }
    serde_json::from_str(&json).map_err(|error| {
        AgentError::new("invalid_planned_test_preparation_json", error.to_string())
    })
}

fn verify_assignment_source(
    definition: &ExecutionConfigurationDefinition,
    preparation: &PlannedTestPreparationDefinition,
) -> Result<(), AgentError> {
    let mut prepared: BTreeSet<_> = preparation
        .station_material_assignments
        .iter()
        .map(|assignment| {
            (
                assignment.requirement_id.as_str(),
                assignment.asset_id.as_str(),
            )
        })
        .collect();
    for assignment in &preparation.assignments {
        if let Some(asset) = preparation
            .station_setup
            .assets
            .iter()
            .find(|asset| asset.binding_id == assignment.binding_id)
        {
            prepared.insert((assignment.slot_id.as_str(), asset.asset_id.as_str()));
        }
    }
    for assignment in &definition.assignments {
        if !prepared.contains(&(
            assignment.requirement_id.as_str(),
            assignment.asset_id.as_str(),
        )) {
            return Err(AgentError::with_details(
                "execution_assignment_not_in_preparation",
                "physical assignments must come from the 0.22.1 planned-test preparation",
                json!({ "role_id": assignment.role_id, "requirement_id": assignment.requirement_id, "asset_id": assignment.asset_id }),
            ));
        }
    }
    Ok(())
}

fn execution_configuration_result(
    connection: &Connection,
    configuration_id: &str,
    replayed: bool,
) -> Result<String, AgentError> {
    let current: Option<(String, String, String, String)> = connection
        .query_row(
            "SELECT r.revision_id, r.definition_json, r.definition_checksum, r.readiness_json FROM execution_configuration_identities i JOIN execution_configuration_revisions r ON r.revision_id = i.current_revision_id WHERE i.configuration_id = ?1",
            params![configuration_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .optional()
        .map_err(sql_error("execution_configuration_query_failed"))?;
    let (revision_id, definition_json, definition_checksum, readiness_json) =
        current.ok_or_else(|| {
            AgentError::new(
                "execution_configuration_not_found",
                "execution configuration does not exist",
            )
        })?;
    Ok(render_json(&json!({
        "operation": "execution_configuration_derived",
        "replayed": replayed,
        "configuration_id": configuration_id,
        "revision_id": revision_id,
        "definition_checksum": definition_checksum,
        "definition": serde_json::from_str::<Value>(&definition_json).unwrap_or(Value::Null),
        "readiness": serde_json::from_str::<Value>(&readiness_json).unwrap_or(Value::Null),
    })))
}

fn load_replay(
    connection: &Connection,
    operation_id: &str,
    request_checksum: &str,
) -> Result<Option<String>, AgentError> {
    let stored: Option<(String, String)> = connection
        .query_row(
            "SELECT request_checksum, configuration_id FROM execution_configuration_revisions WHERE operation_id = ?1",
            params![operation_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(sql_error("execution_configuration_query_failed"))?;
    match stored {
        Some((stored_checksum, configuration_id)) if stored_checksum == request_checksum => {
            execution_configuration_result(connection, &configuration_id, true).map(Some)
        }
        Some((stored_checksum, _)) => Err(AgentError::with_details(
            "operation_replay_mismatch",
            "operation_id is already used for another execution configuration",
            json!({ "operation_id": operation_id, "stored_checksum": stored_checksum, "request_checksum": request_checksum }),
        )),
        None => Ok(None),
    }
}

fn validate_input(input: &DeriveExecutionConfigurationInput) -> Result<(), AgentError> {
    for (field, value) in [
        ("project_code", input.project_code.as_str()),
        ("schedule_item_code", input.schedule_item_code.as_str()),
        (
            "planned_preparation_revision_id",
            input.planned_preparation_revision_id.as_str(),
        ),
        ("actor", input.context.actor.as_str()),
        ("reason", input.context.reason.as_str()),
        ("operation_id", input.context.operation_id.as_str()),
        ("device_id", input.context.device_id.as_str()),
        ("correlation_id", input.context.correlation_id.as_str()),
    ] {
        if value.trim().is_empty() {
            return Err(AgentError::with_details(
                "invalid_execution_configuration_request",
                format!("{field} is required"),
                json!({ "field": field }),
            ));
        }
    }
    Ok(())
}

fn validation_error(error: emc_locus_core::MethodWorkflowValidationIssue) -> AgentError {
    AgentError::with_details(
        "invalid_execution_configuration",
        error.message,
        json!({ "validation_code": error.code, "path": error.path }),
    )
}

fn operation_checksum(payload: &str, context: &ExecutionConfigurationOperationContext) -> String {
    let fingerprint = render_json(&json!({
        "payload": serde_json::from_str::<Value>(payload).expect("canonical execution payload"),
        "actor": context.actor,
        "device_id": context.device_id,
        "correlation_id": context.correlation_id,
    }));
    let digest = Sha256::digest(fingerprint.as_bytes());
    format!("sha256:{digest:x}")
}

fn utc_timestamp() -> Result<String, AgentError> {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .map_err(|error| AgentError::new("timestamp_format_error", error.to_string()))
}

fn sql_error(code: &'static str) -> impl FnOnce(rusqlite::Error) -> AgentError {
    move |error| AgentError::new(code, error.to_string())
}
